use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use oxeylyzer_core::{
    data::Data,
    fast_layout::{BigramPair, FastLayout},
    generate::{LayoutStats, Oxeylyzer},
    layout::{Layout, LayoutMetadata},
    rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    weights::{Config, FingerWeights, MaxFingerUse, Weights},
};
use oxeylyzer_resources::OxeylyzerDirs;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

mod generated_layout;

// ─── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct LayoutStatsDto {
    pub sfb: f64,
    pub dsfb: f64,
    pub fspeed: f64,
    pub finger_speed: [f64; 10],
    /// Per-finger usage as % of all keystrokes (LP..RP order, matching finger_speed).
    pub finger_usage: [f64; 10],
    pub stretches: f64,
    pub scissors: f64,
    pub lsbs: f64,
    pub pinky_ring: f64,
    pub score: f64,
    // trigram fields, flattened to match frontend LayoutStats type
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub bad_sfbs: f64,
    pub sfts: f64,
}

#[derive(Serialize, Clone)]
pub struct LayoutDto {
    pub name: String,
    pub keys: String,
    pub board: String,
    pub fingering_name: Option<String>,
    pub stats: LayoutStatsDto,
    /// Physical key geometry: [x, y, width, height] per key (flat, same order as keys)
    pub keyboard: Vec<[f64; 4]>,
    /// Number of keys per row
    pub shape: Vec<usize>,
}

#[derive(Serialize, Clone)]
pub struct BigramEntryDto {
    pub bigram: String,
    pub percent: f64,
}

#[derive(Serialize, Clone)]
pub struct TrigramEntryDto {
    pub trigram: String,
    pub percent: f64,
}

#[derive(Serialize, Clone)]
pub struct CharFreqDto {
    pub char: String,
    pub percent: f64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NgramResultDto {
    Unigram {
        #[serde(rename = "char")]
        ch: String,
        percent: f64,
    },
    Bigram {
        bigram: String,
        rev: String,
        total: f64,
        fwd: f64,
        bwd: f64,
        #[serde(rename = "skipTotal")]
        skip_total: f64,
        #[serde(rename = "skipFwd")]
        skip_fwd: f64,
        #[serde(rename = "skipBwd")]
        skip_bwd: f64,
    },
    Trigram {
        trigram: String,
        percent: f64,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub view: String,
    pub language: String,
    // Option fields default to None when missing, which keeps old session
    // files (snake_case last_layout) loadable — that key is simply ignored.
    pub last_layout: Option<String>,
    #[serde(default)]
    pub heat_scheme: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MaxFingerUseDto {
    pub penalty: f64,
    pub pinky: f64,
    pub ring: f64,
    pub middle: f64,
    pub index: f64,
    pub thumb: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeightsDto {
    pub lateral_penalty: f64,
    pub sfbs: f64,
    pub sfs: f64,
    pub stretches: f64,
    pub pinky_ring_bigrams: f64,
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub finger_weights: FingerWeights,
    pub max_finger_use: MaxFingerUseDto,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigDto {
    pub corpus: String,
    pub layouts: Vec<String>,
    pub corpus_configs: String,
    pub trigram_precision: usize,
    pub max_cores: usize,
    pub weights: WeightsDto,
}

#[derive(Serialize)]
pub struct ConfigSnapshotDto {
    pub config: ConfigDto,
    /// Exact loaded bytes identify the form's base without a second revision counter.
    pub revision: Vec<u8>,
}

// ─── App State ────────────────────────────────────────────────────────────────

pub struct AppState {
    /// Acquire before other state locks for transition preparation and publication.
    pub transition: Mutex<()>,
    /// Orders frontend results against engine changes, including delayed events.
    pub engine_revision: AtomicU64,
    pub engine_changed: Box<dyn Fn(u64) + Send + Sync>,
    /// Effective corpus path for this session, independent of persisted config.
    pub active_corpus: Mutex<PathBuf>,
    /// Resolved data directory used to prepare the active engine and layouts.
    pub active_data_root: Mutex<PathBuf>,
    /// Last successfully loaded or saved bytes, used to reject known external edits.
    pub config_contents: Mutex<Vec<u8>>,
    /// The active analyzer engine, wrapped in Arc so it can be cheaply cloned for
    /// background generation without holding the lock.
    pub engine: Mutex<Arc<Oxeylyzer>>,
    /// All loaded layouts, keyed by lowercase name.
    pub layouts: Mutex<HashMap<String, Layout>>,
    /// Results from the most recent generation run. Cleared when the engine
    /// changes (language switch, config change) since the layouts are tied to
    /// the engine's character mapping.
    pub generated: Mutex<Vec<FastLayout>>,
    /// Managed resource paths (XDG/AppData config dir in release, override in dev).
    pub dirs: OxeylyzerDirs,
    /// Cached config for reload and language switching.
    pub config: Mutex<Config>,
    /// Set to true to request cancellation of an in-progress generation.
    pub cancel_flag: Arc<AtomicBool>,
    /// True while a generation run is in progress; prevents overlapping runs.
    pub generating: Arc<AtomicBool>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn normalize_score(raw: i64, char_total: i64) -> f64 {
    if char_total == 0 {
        return 0.0;
    }
    (raw as f64) / (char_total as f64) / 100.0
}

/// Per-finger usage as % of all keystrokes, in LP..RP order.
fn finger_usage_pct(engine: &Oxeylyzer, fl: &FastLayout) -> [f64; 10] {
    let total = engine.data.char_total as f64;
    let mut usage = [0.0f64; 10];
    if total == 0.0 {
        return usage;
    }
    for (i, &u) in fl.keys.iter().enumerate() {
        let finger = fl.fingers[i] as usize;
        usage[finger] += engine.data.chars()[u as usize] as f64;
    }
    usage.map(|v| v / total * 100.0)
}

fn stats_to_dto(stats: &LayoutStats, char_total: i64, finger_usage: [f64; 10]) -> LayoutStatsDto {
    let t = &stats.trigram_stats;
    LayoutStatsDto {
        sfb: stats.sfb,
        dsfb: stats.dsfb,
        fspeed: stats.fspeed,
        finger_speed: stats.finger_speed,
        finger_usage,
        stretches: stats.stretches,
        scissors: stats.scissors,
        lsbs: stats.lsbs,
        pinky_ring: stats.pinky_ring,
        score: normalize_score(stats.score, char_total),
        inrolls: t.inrolls,
        outrolls: t.outrolls,
        onehands: t.onehands,
        alternates: t.alternates,
        alternates_sfs: t.alternates_sfs,
        redirects: t.redirects,
        redirects_sfs: t.redirects_sfs,
        bad_redirects: t.bad_redirects,
        bad_redirects_sfs: t.bad_redirects_sfs,
        bad_sfbs: t.bad_sfbs,
        sfts: t.sfts,
    }
}

fn layout_to_dto(engine: &Oxeylyzer, layout: &Layout) -> LayoutDto {
    let fast = engine.fast_layout(layout, &[]);
    let stats = engine.get_layout_stats(&fast);
    let stats_dto = stats_to_dto(
        &stats,
        engine.data.char_total,
        finger_usage_pct(engine, &fast),
    );
    LayoutDto {
        name: layout.name.clone(),
        keys: fast.layout_str(),
        board: get_board_str(layout),
        fingering_name: layout
            .metadata
            .fingering_name
            .as_ref()
            .map(|n| n.to_string()),
        stats: stats_dto,
        keyboard: fast
            .keyboard
            .iter()
            .map(|k| [k.x(), k.y(), k.width(), k.height()])
            .collect(),
        shape: fast.shape.inner().to_vec(),
    }
}

fn get_board_str(layout: &Layout) -> String {
    serde_json::to_value(layout)
        .ok()
        .and_then(|v| v.get("board")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn load_all_layouts(config: &Config, base_path: &Path) -> (HashMap<String, Layout>, Vec<String>) {
    let mut layouts = HashMap::new();
    let mut errors = Vec::new();
    for pattern in &config.layouts {
        let full = base_path.join(pattern);
        for path in glob::glob(&full.to_string_lossy())
            .into_iter()
            .flatten()
            .flatten()
        {
            match Layout::load(&path) {
                Ok(layout) => {
                    layouts.insert(layout.name.to_lowercase(), layout);
                }
                Err(error) => errors.push(format!(
                    "Error loading layout '{}': {error}",
                    path.display()
                )),
            }
        }
    }
    (layouts, errors)
}

/// Builds a [`FastLayout`] from a base layout with an optional custom key
/// arrangement and disabled positions applied, rebuilding `char_to_finger`
/// so trigram classification matches the final arrangement.
fn custom_fast_layout(
    engine: &Oxeylyzer,
    layout: &Layout,
    keys: Option<&str>,
    disabled_indices: &[usize],
) -> Result<FastLayout, String> {
    let mut fl = engine.fast_layout(layout, &[]);

    if let Some(keys) = keys {
        let chars: Vec<char> = keys.chars().collect();
        if chars.len() != fl.keys.len() {
            return Err(format!(
                "Key count mismatch: layout has {} keys, got {}",
                fl.keys.len(),
                chars.len()
            ));
        }
        for (i, &c) in chars.iter().enumerate() {
            fl.keys[i] = engine.mapping.get_u(c);
        }
    }

    for &idx in disabled_indices {
        if idx < fl.keys.len() {
            fl.keys[idx] = 0;
        }
    }

    fl.char_to_finger.iter_mut().for_each(|f| *f = None);
    fl.keys.iter().enumerate().for_each(|(i, &c)| {
        if c != 0 {
            fl.char_to_finger[c as usize] = Some(fl.fingers[i]);
        }
    });

    Ok(fl)
}

fn pin_positions(fast: &FastLayout, engine: &Oxeylyzer, pins: &str) -> Vec<usize> {
    let pin_set: std::collections::HashSet<char> = pins.chars().collect();
    fast.keys
        .iter()
        .map(|&u| engine.mapping.get_c(u))
        .enumerate()
        .filter_map(|(i, c)| pin_set.contains(&c).then_some(i))
        .collect()
}

fn bigram_str(engine: &Oxeylyzer, fl: &FastLayout, pair: &BigramPair) -> Option<String> {
    let u1 = fl.char(pair.pair.0)?;
    let u2 = fl.char(pair.pair.1)?;
    Some(engine.mapping.map_us(&[u1, u2]).collect())
}

fn list_languages_from_dir(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                Some(name.trim_end_matches(".json").to_string())
            } else {
                None
            }
        })
        .collect()
}

fn corpus_path_for(language_data_dir: &Path, language: &str) -> PathBuf {
    language_data_dir.join(language).with_extension("json")
}

fn layout_snapshot(state: &AppState, name: &str) -> Result<(Arc<Oxeylyzer>, Layout), String> {
    let _transition = state.transition.lock().unwrap();
    let engine = state.engine.lock().unwrap().clone();
    let layout = state
        .layouts
        .lock()
        .unwrap()
        .get(&name.to_lowercase())
        .cloned()
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    Ok((engine, layout))
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

fn store_state_snapshot(state: &AppState) -> (Arc<Oxeylyzer>, Vec<Layout>) {
    let _transition = state.transition.lock().unwrap();
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap().values().cloned().collect();
    (engine, layouts)
}

#[tauri::command]
fn list_layouts(state: tauri::State<'_, AppState>) -> Result<Vec<LayoutDto>, String> {
    let (engine, layouts) = store_state_snapshot(&state);
    let mut dtos: Vec<LayoutDto> = layouts.iter().map(|l| layout_to_dto(&engine, l)).collect();
    dtos.sort_by(|a, b| b.stats.score.total_cmp(&a.stats.score));
    Ok(dtos)
}

#[tauri::command]
fn list_languages(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(list_languages_from_dir(&state.dirs.language_data_dir()))
}

#[tauri::command]
fn current_language(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.engine.lock().unwrap().language.clone())
}

#[tauri::command]
fn analyze_layout(name: String, state: tauri::State<'_, AppState>) -> Result<LayoutDto, String> {
    let (engine, layout) = layout_snapshot(&state, &name)?;
    Ok(layout_to_dto(&engine, &layout))
}

#[tauri::command]
fn get_bigrams(
    name: String,
    category: String,
    count: usize,
    keys: Option<String>,
    disabled_indices: Option<Vec<usize>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BigramEntryDto>, String> {
    let (engine, layout) = layout_snapshot(&state, &name)?;
    let fl = custom_fast_layout(
        &engine,
        &layout,
        keys.as_deref(),
        &disabled_indices.unwrap_or_default(),
    )?;
    let bigram_total = engine.data.bigram_total as f64;

    let mut entries: Vec<BigramEntryDto> = match category.as_str() {
        "sfbs" => fl
            .fspeed_indices
            .all
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_sfb(&fl, pair);
                Some(BigramEntryDto {
                    bigram,
                    percent: (raw as f64 * 100.0) / bigram_total,
                })
            })
            .collect(),
        "scissors" => fl
            .scissor_indices
            .pairs
            .iter()
            .filter_map(|&pos_pair| {
                let pair = BigramPair {
                    pair: pos_pair,
                    dist: 1,
                };
                let bigram = bigram_str(&engine, &fl, &pair)?;
                let raw = engine.pair_sfb(&fl, &pair);
                Some(BigramEntryDto {
                    bigram,
                    percent: (raw as f64 * 100.0) / bigram_total,
                })
            })
            .collect(),
        "lsbs" => fl
            .lsb_indices
            .pairs
            .iter()
            .filter_map(|&pos_pair| {
                let pair = BigramPair {
                    pair: pos_pair,
                    dist: 1,
                };
                let bigram = bigram_str(&engine, &fl, &pair)?;
                let raw = engine.pair_sfb(&fl, &pair);
                Some(BigramEntryDto {
                    bigram,
                    percent: (raw as f64 * 100.0) / bigram_total,
                })
            })
            .collect(),
        "pinky-ring" => fl
            .pinky_ring_indices
            .pairs
            .iter()
            .filter_map(|&pos_pair| {
                let pair = BigramPair {
                    pair: pos_pair,
                    dist: 1,
                };
                let bigram = bigram_str(&engine, &fl, &pair)?;
                let raw = engine.pair_sfb(&fl, &pair);
                Some(BigramEntryDto {
                    bigram,
                    percent: (raw as f64 * 100.0) / bigram_total,
                })
            })
            .collect(),
        "fspeed" => fl
            .fspeed_indices
            .all
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_fspeed(&fl, pair).abs();
                Some(BigramEntryDto {
                    bigram,
                    percent: raw as f64 / bigram_total,
                })
            })
            .collect(),
        "stretches" => fl
            .stretch_indices
            .all_pairs
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_stretch(&fl, pair).abs();
                Some(BigramEntryDto {
                    bigram,
                    percent: raw as f64 / bigram_total,
                })
            })
            .collect(),
        other => return Err(format!("Unknown bigram category: '{other}'")),
    };

    entries.sort_by(|a, b| b.percent.total_cmp(&a.percent));
    entries.truncate(count);
    Ok(entries)
}

#[tauri::command]
fn get_trigrams(
    name: String,
    category: String,
    count: usize,
    keys: Option<String>,
    disabled_indices: Option<Vec<usize>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TrigramEntryDto>, String> {
    use oxeylyzer_core::trigram_patterns::TrigramPattern;

    let (engine, layout) = layout_snapshot(&state, &name)?;
    let fl = custom_fast_layout(
        &engine,
        &layout,
        keys.as_deref(),
        &disabled_indices.unwrap_or_default(),
    )?;

    let wanted: &[TrigramPattern] = match category.as_str() {
        "inrolls" => &[TrigramPattern::Inroll],
        "outrolls" => &[TrigramPattern::Outroll],
        "onehands" => &[TrigramPattern::Onehand],
        "alternates" => &[TrigramPattern::Alternate, TrigramPattern::AlternateSfs],
        "redirects" => &[
            TrigramPattern::Redirect,
            TrigramPattern::RedirectSfs,
            TrigramPattern::BadRedirect,
            TrigramPattern::BadRedirectSfs,
        ],
        "sfts" => &[TrigramPattern::Sft],
        other => return Err(format!("Unknown trigram category: '{other}'")),
    };

    let trigram_total = engine.data.trigram_total as f64;
    // gen_trigrams is sorted by frequency descending, so the first `count`
    // matches are already the most frequent ones.
    let entries: Vec<TrigramEntryDto> = engine
        .data
        .gen_trigrams()
        .iter()
        .filter(|(t, _)| wanted.contains(&engine.get_trigram_pattern(&fl, t)))
        .take(count)
        .map(|&(t, freq)| TrigramEntryDto {
            trigram: engine.mapping.map_us(&t).collect(),
            percent: freq as f64 / trigram_total * 100.0,
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
fn swap_keys(
    name: String,
    swaps: String,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let (engine, layout) = layout_snapshot(&state, &name)?;
    let mut fl = engine.fast_layout(&layout, &[]);

    // Parse swap string: space-separated tokens, each token is 2+ chars.
    // "ab" = swap a and b; "abc" = cycle a→b→c.
    for token in swaps.split_whitespace() {
        let chars: Vec<char> = token.chars().collect();
        if chars.len() < 2 {
            continue;
        }
        for window in chars.windows(2) {
            let (c1, c2) = (window[0], window[1]);
            let p1 = fl.keys.iter().position(|&k| k == engine.mapping.get_u(c1));
            let p2 = fl.keys.iter().position(|&k| k == engine.mapping.get_u(c2));
            if let (Some(p1), Some(p2)) = (p1, p2) {
                fl.swap(p1 as u8, p2 as u8);
            }
        }
    }

    let stats = engine.get_layout_stats(&fl);
    let stats_dto = stats_to_dto(
        &stats,
        engine.data.char_total,
        finger_usage_pct(&engine, &fl),
    );
    Ok(LayoutDto {
        name: format!("{name}*"),
        keys: fl.layout_str(),
        board: get_board_str(&layout),
        fingering_name: layout
            .metadata
            .fingering_name
            .as_ref()
            .map(|n| n.to_string()),
        stats: stats_dto,
        keyboard: fl
            .keyboard
            .iter()
            .map(|k| [k.x(), k.y(), k.width(), k.height()])
            .collect(),
        shape: fl.shape.inner().to_vec(),
    })
}

/// Analyze an arbitrary key arrangement (swaps + disabled keys) derived from a named base layout.
/// `keys` is the full 30-char current arrangement; `disabled_indices` are zeroed out before scoring.
/// Returns `keys` unchanged so the frontend always has the clean arrangement available.
#[tauri::command]
fn analyze_custom(
    name: String,
    keys: String,
    disabled_indices: Vec<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let (engine, layout) = layout_snapshot(&state, &name)?;
    let fl = custom_fast_layout(&engine, &layout, Some(&keys), &disabled_indices)?;

    let keyboard = fl
        .keyboard
        .iter()
        .map(|k| [k.x(), k.y(), k.width(), k.height()])
        .collect();
    let shape = fl.shape.inner().to_vec();

    let stats = engine.get_layout_stats(&fl);
    let stats_dto = stats_to_dto(
        &stats,
        engine.data.char_total,
        finger_usage_pct(&engine, &fl),
    );
    Ok(LayoutDto {
        name: format!("{name}*"),
        keys,
        board: get_board_str(&layout),
        fingering_name: layout
            .metadata
            .fingering_name
            .as_ref()
            .map(|n| n.to_string()),
        stats: stats_dto,
        keyboard,
        shape,
    })
}

#[tauri::command]
fn analyze_with_disabled(
    name: String,
    disabled_indices: Vec<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let (engine, layout) = layout_snapshot(&state, &name)?;
    let original_keys = engine.fast_layout(&layout, &[]).layout_str();
    let fl = custom_fast_layout(&engine, &layout, None, &disabled_indices)?;
    let keyboard = fl
        .keyboard
        .iter()
        .map(|k| [k.x(), k.y(), k.width(), k.height()])
        .collect();
    let shape = fl.shape.inner().to_vec();

    let stats = engine.get_layout_stats(&fl);
    let stats_dto = stats_to_dto(
        &stats,
        engine.data.char_total,
        finger_usage_pct(&engine, &fl),
    );
    Ok(LayoutDto {
        name: format!("{name}*"),
        keys: original_keys,
        board: get_board_str(&layout),
        fingering_name: layout
            .metadata
            .fingering_name
            .as_ref()
            .map(|n| n.to_string()),
        stats: stats_dto,
        keyboard,
        shape,
    })
}

#[tauri::command]
fn get_char_frequencies(state: tauri::State<'_, AppState>) -> Result<Vec<CharFreqDto>, String> {
    let engine = state.engine.lock().unwrap().clone();
    Ok(char_frequencies(&engine))
}

fn char_frequencies(engine: &Oxeylyzer) -> Vec<CharFreqDto> {
    let total = engine.data.char_total as f64;
    if total == 0.0 {
        return vec![];
    }
    let freqs: Vec<CharFreqDto> = (0..engine.data.len())
        .filter_map(|i| {
            let c = engine.mapping.get_c(i as u8);
            // Skip the three special chars at indices 0-2
            if c == char::REPLACEMENT_CHARACTER
                || c == oxeylyzer_core::SHIFT_CHAR
                || c == oxeylyzer_core::SPACE_CHAR
            {
                return None;
            }
            let count = engine.data.chars()[i] as f64;
            Some(CharFreqDto {
                char: c.to_string(),
                percent: count / total * 100.0,
            })
        })
        .collect();
    freqs
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoreSnapshotDto {
    layouts: Vec<LayoutDto>,
    languages: Vec<String>,
    current_language: String,
    char_frequencies: HashMap<String, f64>,
}

#[tauri::command]
fn get_store_snapshot(state: tauri::State<'_, AppState>) -> StoreSnapshotDto {
    let (engine, layouts) = store_state_snapshot(&state);
    let mut layouts: Vec<_> = layouts
        .iter()
        .map(|layout| layout_to_dto(&engine, layout))
        .collect();
    layouts.sort_by(|a, b| b.stats.score.total_cmp(&a.stats.score));
    StoreSnapshotDto {
        layouts,
        languages: list_languages_from_dir(&state.dirs.language_data_dir()),
        current_language: engine.language.clone(),
        char_frequencies: char_frequencies(&engine)
            .into_iter()
            .map(|entry| (entry.char, entry.percent))
            .collect(),
    }
}

#[tauri::command]
fn set_language(language: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    set_language_state(&state, &language)
}

fn set_language_state(state: &AppState, language: &str) -> Result<(), String> {
    let _transition = state.transition.lock().unwrap();
    let config = state.config.lock().unwrap().clone();
    let corpus = corpus_path_for(&state.dirs.language_data_dir(), language);
    let (engine, layouts, data_root) = prepare_state(state, &config, &corpus)?;
    publish_prepared(state, config, corpus, engine, layouts, data_root);
    Ok(())
}

#[tauri::command]
fn reload_config(state: tauri::State<'_, AppState>) -> Result<(), String> {
    reload_state(&state)
}

#[tauri::command]
fn lookup_ngram(
    ngram: String,
    state: tauri::State<'_, AppState>,
) -> Result<NgramResultDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let data = &engine.data;

    // get_u maps unknown characters to byte 0 (the replacement character);
    // report those instead of silently returning the replacement's stats.
    for c in ngram.chars() {
        if data.mapping.get_u(c) == 0 {
            return Err(format!("'{c}' does not appear in the current corpus."));
        }
    }

    match ngram.chars().count() {
        1 => {
            let c = ngram.chars().next().unwrap();
            let u = data.mapping.get_u(c);
            let percent = (data.get_char_u(u) as f64 / data.char_total as f64) * 100.0;
            Ok(NgramResultDto::Unigram {
                ch: c.to_string(),
                percent,
            })
        }
        2 => {
            let chars: Vec<char> = ngram.chars().collect();
            let (c1, c2) = (chars[0], chars[1]);
            let u1 = data.mapping.get_u(c1);
            let u2 = data.mapping.get_u(c2);
            let rev: String = [c2, c1].iter().collect();

            let fwd = (data.get_bigram_u([u1, u2]) as f64 / data.bigram_total as f64) * 100.0;
            let bwd = (data.get_bigram_u([u2, u1]) as f64 / data.bigram_total as f64) * 100.0;
            let skip_fwd =
                (data.get_skipgram_u([u1, u2]) as f64 / data.skipgram_total as f64) * 100.0;
            let skip_bwd =
                (data.get_skipgram_u([u2, u1]) as f64 / data.skipgram_total as f64) * 100.0;

            Ok(NgramResultDto::Bigram {
                bigram: ngram,
                rev,
                total: fwd + bwd,
                fwd,
                bwd,
                skip_total: skip_fwd + skip_bwd,
                skip_fwd,
                skip_bwd,
            })
        }
        3 => {
            let chars: Vec<char> = ngram.chars().collect();
            let t = [
                data.mapping.get_u(chars[0]),
                data.mapping.get_u(chars[1]),
                data.mapping.get_u(chars[2]),
            ];
            let &(_, occ) = data
                .gen_trigrams()
                .iter()
                .find(|&&(tf, _)| tf == t)
                .unwrap_or(&(t, 0));
            let percent = (occ as f64) / (data.trigram_total as f64) * 100.0;
            Ok(NgramResultDto::Trigram {
                trigram: ngram,
                percent,
            })
        }
        n => Err(format!(
            "Invalid ngram length {n}. Allowed: 1, 2, or 3 characters."
        )),
    }
}

#[tauri::command]
async fn load_corpus(
    language: String,
    raw: bool,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    use oxeylyzer_core::corpus_cleaner::CorpusCleaner;

    let source_dir = state.dirs.text_dir().join(&language);
    let out_dir = state.dirs.language_data_dir();
    if !source_dir.is_dir() {
        return Err(format!(
            "Source directory '{}' not found.",
            source_dir.display()
        ));
    }

    // Corpus processing is CPU-heavy and can take a long time for large texts —
    // run it on a blocking thread so the UI stays responsive.
    tauri::async_runtime::spawn_blocking(move || {
        let paths: Vec<PathBuf> = std::fs::read_dir(&source_dir)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();

        let cleaner = if raw {
            CorpusCleaner::raw()
        } else {
            CorpusCleaner::default()
        };

        let data = Data::from_paths(&paths, &language, &cleaner)
            .map_err(|e| format!("Failed to process corpus: {e}"))?;

        data.save(&out_dir)
            .map_err(|e| format!("Failed to save corpus: {e}"))?;

        Ok(format!(
            "Corpus '{language}' processed and saved successfully."
        ))
    })
    .await
    .map_err(|e| format!("Corpus task failed: {e}"))?
}

/// Generates one batch of layouts with the selected search algorithm.
/// Defaults follow the empirically tuned parameters from the bench suite
/// (see oxeylyzer-bench): ils (5, 25), sa (0.9997, 50k), lahc (1000, 100k).
fn generate_batch(
    engine: &Oxeylyzer,
    algorithm: &str,
    n: usize,
    basis: &FastLayout,
    pins: &[usize],
) -> Vec<FastLayout> {
    use oxeylyzer_core::generate::{
        annealing::SimulatedAnnealing, engine::Engine, hill_climber::CachedHillClimber,
        ils::IteratedLocalSearch, lahc::LateAcceptanceHillClimbing,
    };

    match algorithm {
        "ils" => IteratedLocalSearch {
            analyzer: engine,
            perturb_swaps: 5,
            rounds: 25,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        "sa" => SimulatedAnnealing {
            analyzer: engine,
            cooling: 0.9997,
            iters: 50_000,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        "lahc" => LateAcceptanceHillClimbing {
            analyzer: engine,
            history: 1_000,
            iters: 100_000,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        _ => CachedHillClimber { analyzer: engine }
            .generate_n_with_pins_iter(n, basis, pins)
            .collect(),
    }
}

#[tauri::command]
async fn start_generate(
    base_layout: String,
    count: usize,
    pins: String,
    algorithm: Option<String>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _transition = state.transition.lock().unwrap();
    // Reject overlapping runs — two concurrent runs would race for
    // `state.generated` and double CPU usage.
    if state
        .generating
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("A generation run is already in progress.".to_string());
    }
    state.cancel_flag.store(false, Ordering::Relaxed);

    // Clone what we need before spawning — avoids moving tauri::State into a thread.
    let engine = state.engine.lock().unwrap().clone();
    let revision = state.engine_revision.load(Ordering::Relaxed);
    let cancel = state.cancel_flag.clone();
    let generating = state.generating.clone();
    let algorithm = algorithm.unwrap_or_else(|| "hill".to_string());

    let fast_base = {
        let layouts = state.layouts.lock().unwrap();
        let base = match layouts.get(&base_layout.to_lowercase()) {
            Some(base) => base,
            None => {
                generating.store(false, Ordering::SeqCst);
                return Err(format!("Layout '{base_layout}' not found"));
            }
        };
        engine.fast_layout(base, &[])
    };

    let pin_pos = pin_positions(&fast_base, &engine, &pins);
    let app = app_handle.clone();

    std::thread::spawn(move || {
        // Generate in parallel batches, checking the cancel flag between
        // batches — this is what makes cancellation actually stop the work
        // instead of merely discarding its results.
        let batch = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let mut results: Vec<FastLayout> = Vec::with_capacity(count);
        let mut last_emit = std::time::Instant::now();

        while results.len() < count && !cancel.load(Ordering::Relaxed) {
            let n = batch.min(count - results.len());
            results.extend(generate_batch(&engine, &algorithm, n, &fast_base, &pin_pos));

            if last_emit.elapsed() >= std::time::Duration::from_millis(200) {
                last_emit = std::time::Instant::now();
                let _ = app.emit(
                    "generate-progress",
                    serde_json::json!({ "done": results.len(), "total": count }),
                );
            }
        }

        let cancelled = cancel.load(Ordering::Relaxed);
        {
            let state = app.state::<AppState>();
            let _transition = state.transition.lock().unwrap();
            if !Arc::ptr_eq(&engine, &state.engine.lock().unwrap()) {
                results.clear();
            }
        }

        // Pre-compute scores once per layout, then sort by cached value.
        // Avoids calling engine.score() O(N log N) times inside the comparator.
        let char_total = engine.data.char_total;
        let mut scored: Vec<(i64, FastLayout)> = results
            .into_iter()
            .map(|fl| (engine.score(&fl), fl))
            .collect();
        scored.sort_unstable_by(|(s1, _), (s2, _)| s2.cmp(s1));

        // Build DTOs for the top 50 in parallel — get_layout_stats is expensive
        // but read-only, so rayon can safely run it across threads.
        let top = &scored[..50.min(scored.len())];
        let layout_dtos: Vec<LayoutDto> = top
            .par_iter()
            .enumerate()
            .map(|(i, (_, fl))| {
                let stats = engine.get_layout_stats(fl);
                let stats_dto = stats_to_dto(&stats, char_total, finger_usage_pct(&engine, fl));
                LayoutDto {
                    name: fl.name.clone().unwrap_or_else(|| format!("gen-{}", i + 1)),
                    keys: fl.layout_str(),
                    board: "generated".to_string(),
                    fingering_name: fl.metadata.fingering_name.as_ref().map(|n| n.to_string()),
                    stats: stats_dto,
                    keyboard: fl
                        .keyboard
                        .iter()
                        .map(|k| [k.x(), k.y(), k.width(), k.height()])
                        .collect(),
                    shape: fl.shape.inner().to_vec(),
                }
            })
            .collect();

        // Store all results (sorted) for save_generated. Partial results from a
        // cancelled run are kept on purpose — they're still valid layouts.
        let state = app.state::<AppState>();
        finish_generation(
            &state,
            &engine,
            revision,
            scored.into_iter().map(|(_, fl)| fl).collect(),
            layout_dtos,
            cancelled,
            |payload| {
                let _ = app.emit("generate-done", payload);
            },
        );
    });

    Ok(())
}

fn finish_generation(
    state: &AppState,
    engine: &Arc<Oxeylyzer>,
    revision: u64,
    results: Vec<FastLayout>,
    layout_dtos: Vec<LayoutDto>,
    cancelled: bool,
    publish: impl FnOnce(serde_json::Value),
) {
    // Keep validity, saveable results, and event publication in one engine transition.
    let _transition = state.transition.lock().unwrap();
    let stale = !Arc::ptr_eq(engine, &state.engine.lock().unwrap());
    let payload = if stale {
        state.generated.lock().unwrap().clear();
        serde_json::json!({
            "results": [],
            "revision": revision,
            "cancelled": true,
            "reason": "Generation stopped because the language or configuration changed. Please generate again.",
        })
    } else {
        *state.generated.lock().unwrap() = results;
        serde_json::json!({ "results": layout_dtos, "cancelled": cancelled, "reason": null, "revision": revision })
    };
    state.generating.store(false, Ordering::SeqCst);
    publish(payload);
}

#[tauri::command]
fn save_generated(
    index: usize,
    name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let _transition = state.transition.lock().unwrap();
    let engine = state.engine.lock().unwrap().clone();
    let mut generated = state.generated.lock().unwrap();

    let len = generated.len();
    let fl = generated
        .get_mut(index)
        .ok_or_else(|| format!("Index {index} out of bounds ({len} results)"))?;

    // Assign the chosen name.
    let save_name = name.unwrap_or_else(|| {
        fl.keys
            .iter()
            .skip(10)
            .take(4)
            .map(|&u| engine.mapping.get_c(u))
            .collect::<String>()
    });
    validate_filename(&save_name)?;
    fl.name = Some(save_name.clone());

    // Serialize to .dof JSON, clearing provenance fields from the base layout.
    let layout: Layout = fl.clone().into();
    let layout = Layout {
        metadata: Arc::new(LayoutMetadata {
            authors: vec![],
            year: None,
            link: None,
            ..(*layout.metadata).clone()
        }),
        ..layout
    };
    let json =
        serde_json::to_string_pretty(&layout).map_err(|e| format!("Serialization failed: {e}"))?;

    // Write to the layouts directory for the current language.
    let lang = engine.language.clone();
    let path = layout_path(&state.dirs.layouts_dir(), &lang, &save_name)?;

    generated_layout::write_new(&path, &json)?;

    // Add to loaded layouts.
    let layout_loaded = Layout::load(&path).map_err(|e| e.to_string())?;
    let dto = layout_to_dto(&engine, &layout_loaded);
    state
        .layouts
        .lock()
        .unwrap()
        .insert(save_name.to_lowercase(), layout_loaded);

    Ok(dto)
}

#[tauri::command]
fn cancel_generate(state: tauri::State<'_, AppState>) {
    let _transition = state.transition.lock().unwrap();
    state.cancel_flag.store(true, Ordering::Relaxed);
}

fn validate_filename(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
        return Err("Name must be a single filename without path separators".to_string());
    }
    Ok(())
}

fn layout_path(dir: &Path, language: &str, name: &str) -> Result<PathBuf, String> {
    validate_filename(language).map_err(|e| format!("Invalid corpus language: {e}"))?;
    validate_filename(name)?;
    let filename = name.replace(' ', "_").to_lowercase();
    validate_filename(&filename)?;
    Ok(dir.join(language).join(filename).with_extension("dof"))
}

fn preset_path(dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf, String> {
    validate_filename(name)?;
    Ok(dir.join(name).with_extension("toml"))
}

#[tauri::command]
fn delete_layout(name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _transition = state.transition.lock().unwrap();
    let lang = state.engine.lock().unwrap().language.clone();
    delete_layout_file(&state.dirs.layouts_dir(), &lang, &name)?;
    state.layouts.lock().unwrap().remove(&name.to_lowercase());
    Ok(())
}

fn delete_layout_file(dir: &Path, language: &str, name: &str) -> Result<(), String> {
    let path = layout_path(dir, language, name)?;

    if !path.exists() {
        return Err(format!(
            "Layout file '{}' not found — it may live outside the managed layouts directory.",
            path.display()
        ));
    }
    std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))
}

/// Saves a modified key arrangement (e.g. from drag-swaps in the Analyze view)
/// as a new layout derived from `base_name`.
#[tauri::command]
fn save_custom_layout(
    base_name: String,
    keys: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let _transition = state.transition.lock().unwrap();
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("A name is required.".to_string());
    }

    let engine = state.engine.lock().unwrap().clone();
    let mut fl = {
        let layouts = state.layouts.lock().unwrap();
        let base = layouts
            .get(&base_name.to_lowercase())
            .ok_or_else(|| format!("Layout '{base_name}' not found"))?;
        custom_fast_layout(&engine, base, Some(&keys), &[])?
    };
    fl.name = Some(new_name.clone());

    // Serialize to .dof JSON, clearing provenance fields from the base layout.
    let layout: Layout = fl.into();
    let layout = Layout {
        metadata: Arc::new(LayoutMetadata {
            authors: vec![],
            year: None,
            link: None,
            ..(*layout.metadata).clone()
        }),
        ..layout
    };
    let json =
        serde_json::to_string_pretty(&layout).map_err(|e| format!("Serialization failed: {e}"))?;

    let lang = engine.language.clone();
    let path = layout_path(&state.dirs.layouts_dir(), &lang, &new_name)?;

    if path.exists() {
        return Err(format!("A layout named '{new_name}' already exists."));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &json).map_err(|e| format!("Write failed: {e}"))?;

    let layout_loaded = Layout::load(&path).map_err(|e| e.to_string())?;
    let dto = layout_to_dto(&engine, &layout_loaded);
    state
        .layouts
        .lock()
        .unwrap()
        .insert(new_name.to_lowercase(), layout_loaded);

    Ok(dto)
}

#[tauri::command]
fn get_layout_detail(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    serde_json::to_value(layout).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_layout_edit(
    dof_json: serde_json::Value,
    original_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _transition = state.transition.lock().unwrap();
    let layout: Layout = serde_json::from_value(dof_json.clone())
        .map_err(|e| format!("Invalid layout JSON: {e}"))?;
    let new_name = layout.name.clone();

    // Find the original file path.
    let lang = state.engine.lock().unwrap().language.clone();
    let path = layout_path(&state.dirs.layouts_dir(), &lang, &original_name)?;

    let json = serde_json::to_string_pretty(&dof_json).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| format!("Write failed: {e}"))?;

    // Reload into state.
    let layout_loaded = Layout::load(&path).map_err(|e| e.to_string())?;
    let mut layouts = state.layouts.lock().unwrap();
    layouts.remove(&original_name.to_lowercase());
    layouts.insert(new_name.to_lowercase(), layout_loaded);
    Ok(())
}

#[tauri::command]
fn fork_layout(
    name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let _transition = state.transition.lock().unwrap();
    let engine = state.engine.lock().unwrap().clone();
    let lang = engine.language.clone();

    let mut layouts = state.layouts.lock().unwrap();
    let original = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?
        .clone();

    let mut forked = original.clone();
    forked.name = new_name.clone();

    let path = layout_path(&state.dirs.layouts_dir(), &lang, &new_name)?;

    if path.exists() {
        return Err(format!("A layout named '{new_name}' already exists."));
    }

    let json = serde_json::to_string_pretty(&forked).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &json).map_err(|e| format!("Write failed: {e}"))?;

    let dto = layout_to_dto(&engine, &forked);
    layouts.insert(new_name.to_lowercase(), forked);
    Ok(dto)
}

#[tauri::command]
fn get_session(state: tauri::State<'_, AppState>) -> Result<SessionDto, String> {
    let path = state.dirs.session_file();
    if !path.exists() {
        return Ok(SessionDto {
            view: "layouts".to_string(),
            language: state.engine.lock().unwrap().language.clone(),
            last_layout: None,
            heat_scheme: None,
        });
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_session(session: SessionDto, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let path = state.dirs.session_file();
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())
}

// ─── Config Commands ──────────────────────────────────────────────────────────

#[cfg(test)]
fn config_dto_to_toml(dto: &ConfigDto) -> String {
    let layouts_lines = dto
        .layouts
        .iter()
        .map(|p| format!("  {p:?}"))
        .collect::<Vec<_>>()
        .join(",\n");
    let fw = &dto.weights.finger_weights;
    let mfu = &dto.weights.max_finger_use;
    let w = &dto.weights;
    format!(
        "corpus = {:?}\nlayouts = [\n{}\n]\n\
         corpus_configs = {:?}\ntrigram_precision = {}\nmax_cores = {}\n\n\
         [weights]\n\
         lateral_penalty = {}\nsfbs = {}\nsfs = {}\nstretches = {}\n\
         pinky_ring_bigrams = {}\ninrolls = {}\noutrolls = {}\nonehands = {}\n\
         alternates = {}\nalternates_sfs = {}\nredirects = {}\nredirects_sfs = {}\n\
         bad_redirects = {}\nbad_redirects_sfs = {}\n\n\
         [weights.finger_weights]\n\
         lp = {}\nlr = {}\nlm = {}\nli = {}\nlt = {}\n\
         rt = {}\nri = {}\nrm = {}\nrr = {}\nrp = {}\n\n\
         [weights.max_finger_use]\n\
         penalty = {}\npinky = {}\nring = {}\nmiddle = {}\nindex = {}\nthumb = {}\n",
        dto.corpus,
        layouts_lines,
        dto.corpus_configs,
        dto.trigram_precision,
        dto.max_cores,
        w.lateral_penalty,
        w.sfbs,
        w.sfs,
        w.stretches,
        w.pinky_ring_bigrams,
        w.inrolls,
        w.outrolls,
        w.onehands,
        w.alternates,
        w.alternates_sfs,
        w.redirects,
        w.redirects_sfs,
        w.bad_redirects,
        w.bad_redirects_sfs,
        fw.lp,
        fw.lr,
        fw.lm,
        fw.li,
        fw.lt,
        fw.rt,
        fw.ri,
        fw.rm,
        fw.rr,
        fw.rp,
        mfu.penalty,
        mfu.pinky,
        mfu.ring,
        mfu.middle,
        mfu.index,
        mfu.thumb,
    )
}

fn config_to_dto(config: &Config) -> ConfigDto {
    let w = &config.weights;
    ConfigDto {
        corpus: config.corpus.to_string_lossy().into_owned(),
        layouts: config
            .layouts
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        corpus_configs: config.corpus_configs.to_string_lossy().into_owned(),
        trigram_precision: config.trigram_precision,
        max_cores: config.max_cores,
        weights: WeightsDto {
            lateral_penalty: w.lateral_penalty,
            sfbs: w.sfbs,
            sfs: w.sfs,
            stretches: w.stretches,
            pinky_ring_bigrams: w.pinky_ring_bigrams,
            inrolls: w.inrolls,
            outrolls: w.outrolls,
            onehands: w.onehands,
            alternates: w.alternates,
            alternates_sfs: w.alternates_sfs,
            redirects: w.redirects,
            redirects_sfs: w.redirects_sfs,
            bad_redirects: w.bad_redirects,
            bad_redirects_sfs: w.bad_redirects_sfs,
            finger_weights: w.finger_weights.clone(),
            max_finger_use: MaxFingerUseDto {
                penalty: w.max_finger_use.penalty,
                pinky: w.max_finger_use.pinky,
                ring: w.max_finger_use.ring,
                middle: w.max_finger_use.middle,
                index: w.max_finger_use.index,
                thumb: w.max_finger_use.thumb,
            },
        },
    }
}

fn dto_to_weights(w: &WeightsDto) -> Weights {
    Weights {
        lateral_penalty: w.lateral_penalty,
        sfbs: w.sfbs,
        sfs: w.sfs,
        stretches: w.stretches,
        pinky_ring_bigrams: w.pinky_ring_bigrams,
        inrolls: w.inrolls,
        outrolls: w.outrolls,
        onehands: w.onehands,
        alternates: w.alternates,
        alternates_sfs: w.alternates_sfs,
        redirects: w.redirects,
        redirects_sfs: w.redirects_sfs,
        bad_redirects: w.bad_redirects,
        bad_redirects_sfs: w.bad_redirects_sfs,
        finger_weights: w.finger_weights.clone(),
        max_finger_use: MaxFingerUse {
            penalty: w.max_finger_use.penalty,
            pinky: w.max_finger_use.pinky,
            ring: w.max_finger_use.ring,
            middle: w.max_finger_use.middle,
            index: w.max_finger_use.index,
            thumb: w.max_finger_use.thumb,
        },
    }
}

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<ConfigSnapshotDto, String> {
    Ok(get_config_state(&state))
}

fn get_config_state(state: &AppState) -> ConfigSnapshotDto {
    let _transition = state.transition.lock().unwrap();
    ConfigSnapshotDto {
        config: config_to_dto(&state.config.lock().unwrap()),
        revision: state.config_contents.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn set_config(
    config_dto: ConfigDto,
    revision: Vec<u8>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    save_config_state(&state, config_dto, &revision)
}

fn save_config_state(
    state: &AppState,
    config_dto: ConfigDto,
    revision: &[u8],
) -> Result<Vec<u8>, String> {
    let _transition = state.transition.lock().unwrap();
    if revision != *state.config_contents.lock().unwrap() {
        return Err("Config changed since it was loaded; reopen Config before saving.".into());
    }
    let new_weights = dto_to_weights(&config_dto.weights);
    let new_config = Config {
        corpus: PathBuf::from(&config_dto.corpus),
        layouts: config_dto.layouts.iter().map(PathBuf::from).collect(),
        corpus_configs: PathBuf::from(&config_dto.corpus_configs),
        trigram_precision: config_dto.trigram_precision,
        max_cores: config_dto.max_cores,
        weights: new_weights,
    };

    let corpus = if new_config.corpus == state.config.lock().unwrap().corpus {
        state.active_corpus.lock().unwrap().clone()
    } else {
        state.dirs.data_dir().join(&new_config.corpus)
    };
    let contents =
        toml::to_string(&new_config).map_err(|e| format!("Serialize config failed: {e}"))?;
    toml::from_str::<Config>(&contents).map_err(|e| format!("Invalid serialized config: {e}"))?;
    let contents = contents.into_bytes();
    let (engine, layouts, data_root) = prepare_state(state, &new_config, &corpus)?;
    replace_config(
        &state.dirs.config_file(),
        &state.config_contents.lock().unwrap(),
        &contents,
    )?;
    *state.config_contents.lock().unwrap() = contents.clone();
    publish_prepared(state, new_config, corpus, engine, layouts, data_root);
    Ok(contents)
}

#[tauri::command]
fn get_defaults() -> Result<ConfigDto, String> {
    Ok(config_to_dto(&Config::with_defaults()))
}

// ─── Weight Presets ───────────────────────────────────────────────────────────

fn preset_dir(dirs: &OxeylyzerDirs) -> PathBuf {
    dirs.weight_presets_dir()
}

fn weights_dto_to_toml(w: &WeightsDto) -> String {
    let fw = &w.finger_weights;
    let mfu = &w.max_finger_use;
    format!(
        "lateral_penalty = {}\nsfbs = {}\nsfs = {}\nstretches = {}\n\
         pinky_ring_bigrams = {}\ninrolls = {}\noutrolls = {}\nonehands = {}\n\
         alternates = {}\nalternates_sfs = {}\nredirects = {}\nredirects_sfs = {}\n\
         bad_redirects = {}\nbad_redirects_sfs = {}\n\n\
         [finger_weights]\n\
         lp = {}\nlr = {}\nlm = {}\nli = {}\nlt = {}\n\
         rt = {}\nri = {}\nrm = {}\nrr = {}\nrp = {}\n\n\
         [max_finger_use]\n\
         penalty = {}\npinky = {}\nring = {}\nmiddle = {}\nindex = {}\nthumb = {}\n",
        w.lateral_penalty,
        w.sfbs,
        w.sfs,
        w.stretches,
        w.pinky_ring_bigrams,
        w.inrolls,
        w.outrolls,
        w.onehands,
        w.alternates,
        w.alternates_sfs,
        w.redirects,
        w.redirects_sfs,
        w.bad_redirects,
        w.bad_redirects_sfs,
        fw.lp,
        fw.lr,
        fw.lm,
        fw.li,
        fw.lt,
        fw.rt,
        fw.ri,
        fw.rm,
        fw.rr,
        fw.rp,
        mfu.penalty,
        mfu.pinky,
        mfu.ring,
        mfu.middle,
        mfu.index,
        mfu.thumb,
    )
}

#[tauri::command]
fn list_weight_presets(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let dir = preset_dir(&state.dirs);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.ends_with(".toml")
                .then(|| name.trim_end_matches(".toml").to_string())
        })
        .collect();
    names.sort();
    Ok(names)
}

#[tauri::command]
fn save_weight_preset(
    name: String,
    weights: WeightsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    write_weight_preset(&preset_dir(&state.dirs), &name, &weights)
}

fn write_weight_preset(dir: &Path, name: &str, weights: &WeightsDto) -> Result<(), String> {
    let path = preset_path(dir, name)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, weights_dto_to_toml(weights)).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_weight_preset(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<WeightsDto, String> {
    read_weight_preset(&preset_dir(&state.dirs), &name)
}

fn read_weight_preset(dir: &Path, name: &str) -> Result<WeightsDto, String> {
    let path = preset_path(dir, name)?;
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    toml::from_str::<WeightsDto>(&s).map_err(|e| format!("Failed to parse preset '{name}': {e}"))
}

// ─── Reload Helper ────────────────────────────────────────────────────────────

type PreparedState = (Arc<Oxeylyzer>, HashMap<String, Layout>, PathBuf);

fn prepare_state(
    state: &AppState,
    config: &Config,
    corpus: &Path,
) -> Result<PreparedState, String> {
    let data_root = state
        .dirs
        .data_dir()
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let data = Data::load(corpus).map_err(|e| format!("Failed to load corpus: {e}"))?;
    let (layouts, errors) = load_all_layouts(config, state.dirs.data_dir());
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    if state
        .dirs
        .data_dir()
        .canonicalize()
        .map_err(|e| e.to_string())?
        != data_root
    {
        return Err("Data directory changed during reload; retry loading it.".into());
    }
    Ok((
        Arc::new(Oxeylyzer::new(data, config.clone())),
        layouts,
        data_root,
    ))
}

/// Caller holds transition through preparation, persistence, and publication.
fn publish_prepared(
    state: &AppState,
    config: Config,
    corpus: PathBuf,
    engine: Arc<Oxeylyzer>,
    layouts: HashMap<String, Layout>,
    data_root: PathBuf,
) {
    state.cancel_flag.store(true, Ordering::Relaxed);
    *state.config.lock().unwrap() = config;
    *state.active_corpus.lock().unwrap() = corpus;
    *state.active_data_root.lock().unwrap() = data_root;
    *state.engine.lock().unwrap() = engine;
    *state.layouts.lock().unwrap() = layouts;
    state.generated.lock().unwrap().clear();
    let revision = state.engine_revision.fetch_add(1, Ordering::Relaxed) + 1;
    (state.engine_changed)(revision);
}

fn replace_config(path: &Path, expected: &[u8], contents: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::AtomicUsize;
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::fs::canonicalize(path).map_err(|e| format!("Read config failed: {e}"))?;
    let path = path.as_path();
    let metadata = std::fs::metadata(path).map_err(|e| format!("Read config failed: {e}"))?;
    if !metadata.is_file() {
        return Err("Config must be a regular file.".into());
    }
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| format!("Write failed: {e}"))?;
    let (temp, mut file) = loop {
        let temp = path.with_file_name(format!(
            ".config-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Write failed: {e}")),
        }
    };
    let result = (|| {
        file.set_permissions(metadata.permissions())
            .map_err(|e| format!("Set config permissions failed: {e}"))?;
        file.write_all(contents)
            .map_err(|e| format!("Write failed: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Sync config failed: {e}"))?;
        let current = std::fs::read(path).map_err(|e| format!("Read config failed: {e}"))?;
        if current != expected {
            return Err("Config changed externally; reload it before saving.".into());
        }
        // shortcut: external editors can race this final check, use a shared writer protocol if strict CAS is required.
        std::fs::rename(&temp, path).map_err(|e| format!("Replace config failed: {e}"))
    })();
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn reload_state(state: &AppState) -> Result<(), String> {
    let _transition = state.transition.lock().unwrap();
    let contents = std::fs::read(state.dirs.config_file())
        .map_err(|e| format!("Failed to reload config: {e}"))?;
    let data_root = state
        .dirs
        .data_dir()
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if contents == *state.config_contents.lock().unwrap()
        && data_root == *state.active_data_root.lock().unwrap()
    {
        return Ok(());
    }
    let text =
        std::str::from_utf8(&contents).map_err(|e| format!("Failed to reload config: {e}"))?;
    let config: Config =
        toml::from_str(text).map_err(|e| format!("Failed to reload config: {e}"))?;
    let corpus = if config.corpus == state.config.lock().unwrap().corpus {
        state.active_corpus.lock().unwrap().clone()
    } else {
        state.dirs.data_dir().join(&config.corpus)
    };
    let (engine, layouts, data_root) = prepare_state(state, &config, &corpus)?;
    if std::fs::read(state.dirs.config_file())
        .map_err(|e| format!("Failed to reload config: {e}"))?
        != contents
    {
        return Err("Config changed externally during reload; retry loading it.".into());
    }
    *state.config_contents.lock().unwrap() = contents;
    publish_prepared(state, config, corpus, engine, layouts, data_root);
    Ok(())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct WatchChanges {
    config: bool,
    layouts: bool,
}

fn config_watch_path(config: &Path) -> std::io::Result<PathBuf> {
    std::path::absolute(config)
}

// Include each link in the resolution chain, including a temporarily missing one.
fn config_link_paths(config: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut remaining = config.to_path_buf();
    for _ in 0..40 {
        let mut resolved = PathBuf::new();
        let mut components = remaining.components();
        let mut followed = false;
        while let Some(component) = components.next() {
            match component {
                std::path::Component::ParentDir => {
                    resolved.pop();
                }
                std::path::Component::CurDir => continue,
                _ => resolved.push(component.as_os_str()),
            }
            match std::fs::symlink_metadata(&resolved) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    paths.push(resolved.clone());
                    let Ok(link) = std::fs::read_link(&resolved) else {
                        return paths;
                    };
                    let mut next = resolved.parent().unwrap().join(link);
                    next.extend(components.map(|part| part.as_os_str()));
                    remaining = next;
                    followed = true;
                    break;
                }
                Err(_) => {
                    paths.push(resolved);
                    return paths;
                }
                _ => {}
            }
        }
        if !followed {
            break;
        }
    }
    paths
}

fn update_link_watches<W: notify::Watcher>(
    watcher: &mut W,
    config: &Path,
    target: &Path,
    layouts: &Path,
    previous: &mut Vec<PathBuf>,
) -> notify::Result<()> {
    let links = config_link_paths(config);
    let parents: std::collections::HashSet<_> = links.iter().filter_map(|p| p.parent()).collect();
    let reserved =
        |p: &Path| p == config.parent().unwrap() || p == target.parent().unwrap() || p == layouts;
    for parent in &parents {
        if !reserved(parent) {
            watcher.watch(parent, notify::RecursiveMode::NonRecursive)?;
        }
    }
    for parent in previous.iter().filter_map(|p| p.parent()) {
        if !reserved(parent) && !parents.contains(parent) {
            let _ = watcher.unwatch(parent);
        }
    }
    *previous = links;
    Ok(())
}

fn watch_paths(
    tx: std::sync::mpsc::Sender<notify::Result<notify::Event>>,
    config: &Path,
    target: &Path,
    layouts: &Path,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};
    let mut watcher = notify::recommended_watcher(tx)?;
    // Watch the directory so atomic replacement does not detach config monitoring.
    watcher.watch(config.parent().unwrap(), RecursiveMode::NonRecursive)?;
    if target.parent() != config.parent() {
        watcher.watch(target.parent().unwrap(), RecursiveMode::NonRecursive)?;
    }
    if let Err(error) = watcher.watch(layouts, RecursiveMode::Recursive) {
        eprintln!("Layout watcher init failed: {error}");
    }
    Ok(watcher)
}

fn update_config_watch<W: notify::Watcher>(
    watcher: &mut W,
    config: &Path,
    target: &mut PathBuf,
    layouts: &Path,
) -> notify::Result<()> {
    let new_target = match config.canonicalize() {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };
    if new_target == *target {
        return Ok(());
    }
    let old_parent = target.parent().unwrap();
    let new_parent = new_target.parent().unwrap();
    let link_parent = config.parent().unwrap();
    if new_parent != link_parent && new_parent != old_parent && new_parent != layouts {
        watcher.watch(new_parent, notify::RecursiveMode::NonRecursive)?;
    }
    if old_parent != new_parent && old_parent != link_parent && old_parent != layouts {
        let _ = watcher.unwatch(old_parent);
    }
    *target = new_target;
    Ok(())
}

fn update_layout_watch<W: notify::Watcher>(
    watcher: &mut W,
    source: &Path,
    layouts: &mut PathBuf,
    config: &Path,
    target: &Path,
    links: &[PathBuf],
) -> notify::Result<()> {
    let new_layouts = source.canonicalize().map_err(notify::Error::io)?;
    if new_layouts == *layouts {
        return Ok(());
    }
    watcher.watch(&new_layouts, notify::RecursiveMode::Recursive)?;
    if config.parent() == Some(layouts.as_path())
        || target.parent() == Some(layouts.as_path())
        || links
            .iter()
            .any(|link| link.parent() == Some(layouts.as_path()))
    {
        watcher.watch(layouts, notify::RecursiveMode::NonRecursive)?;
    } else {
        let _ = watcher.unwatch(layouts);
    }
    *layouts = new_layouts;
    Ok(())
}

fn next_watch_changes(
    rx: &std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    config: &Path,
    target: &Path,
    layouts: &Path,
    known_links: &[PathBuf],
    quiet: std::time::Duration,
) -> Option<WatchChanges> {
    use std::sync::mpsc::RecvTimeoutError;
    let mut changes = WatchChanges::default();
    let mut deadline: Option<std::time::Instant> = None;
    loop {
        let event = if let Some(until) = deadline {
            let Some(remaining) = until.checked_duration_since(std::time::Instant::now()) else {
                return Some(changes);
            };
            match rx.recv_timeout(remaining) {
                Ok(event) => event,
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    return Some(changes);
                }
            }
        } else {
            rx.recv().ok()?
        };
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                eprintln!("File watcher error: {error}");
                continue;
            }
        };
        if event.need_rescan() {
            changes.config = true;
            changes.layouts = true;
        } else {
            if !matches!(
                event.kind,
                notify::EventKind::Modify(_)
                    | notify::EventKind::Create(_)
                    | notify::EventKind::Remove(_)
            ) {
                continue;
            }
            let links = config_link_paths(config);
            let config_changed = event.paths.iter().any(|path| {
                path == config
                    || path == target
                    || links.contains(path)
                    || known_links.contains(path)
            });
            let layouts_changed = event.paths.iter().any(|path| path.starts_with(layouts));
            if !config_changed && !layouts_changed {
                continue;
            }
            changes.config |= config_changed;
            changes.layouts |= layouts_changed;
        }
        deadline = Some(std::time::Instant::now() + quiet);
    }
}

fn reload_layouts(state: &AppState) -> Result<(), String> {
    // A repaired layout may unblock a config edit that failed preparation.
    if let Err(error) = reload_state(state) {
        eprintln!("Auto-reload failed: {error}");
    }
    let _transition = state.transition.lock().unwrap();
    let config = state.config.lock().unwrap().clone();
    let (layouts, errors) = load_all_layouts(&config, &state.active_data_root.lock().unwrap());
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    *state.layouts.lock().unwrap() = layouts;
    Ok(())
}

// ─── Entry Point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dirs = if let Ok(p) = std::env::var("OXEYLYZER_DATA_DIR") {
                OxeylyzerDirs::with_override(PathBuf::from(p))
            } else {
                OxeylyzerDirs::resolve().expect("failed to resolve data directory")
            };

            // On first run, download data files synchronously before initialising the
            // engine — corpus and layout files must exist before we try to load them.
            // Progress events are emitted so a frontend loading screen can react.
            if dirs.is_first_run() {
                use oxeylyzer_resources::DownloadProgress;
                let app_handle = app.handle().clone();
                dirs.ensure_data(move |p| {
                    let payload = match &p {
                        DownloadProgress::Connecting => {
                            serde_json::json!({"status": "connecting"})
                        }
                        DownloadProgress::Downloading {
                            bytes_done,
                            bytes_total,
                        } => {
                            serde_json::json!({
                                "status": "downloading",
                                "bytesDone": bytes_done,
                                "bytesTotal": bytes_total,
                            })
                        }
                        DownloadProgress::Extracting => {
                            serde_json::json!({"status": "extracting"})
                        }
                        DownloadProgress::Done => serde_json::json!({"status": "done"}),
                    };
                    let _ = app_handle.emit("download-progress", payload);
                })
                .expect("failed to download resources");
            }

            // ensure_config is idempotent; ensure_data already calls it on first run,
            // but call it here too so a missing config is always recovered.
            dirs.ensure_config()
                .expect("failed to write default config");

            let config_contents =
                std::fs::read(dirs.config_file()).expect("failed to read config.toml");
            let config: Config = toml::from_str(
                std::str::from_utf8(&config_contents).expect("config.toml must be UTF-8"),
            )
            .expect("failed to load config.toml");

            let corpus_path = dirs.data_dir().join(&config.corpus);
            let data = Data::load(&corpus_path).expect("failed to load corpus");

            let engine = Arc::new(Oxeylyzer::new(data, config.clone()));
            let (layouts, errors) = load_all_layouts(&config, dirs.data_dir());
            for error in errors {
                eprintln!("{error}");
            }

            // Notify emits absolute paths; resolve overrides such as OXEYLYZER_DATA_DIR=.
            let config_path = dirs.config_file();
            let watch_config = config_watch_path(&config_path)?;
            let watch_target = config_path.canonicalize()?;
            let layouts_path = std::path::absolute(dirs.layouts_dir())?;
            let watch_layouts = layouts_path
                .canonicalize()
                .unwrap_or_else(|_| layouts_path.clone());

            app.manage(AppState {
                transition: Mutex::new(()),
                engine_revision: AtomicU64::new(0),
                engine_changed: Box::new({
                    let app = app.handle().clone();
                    move |revision| {
                        let _ = app.emit("engine-changed", revision);
                    }
                }),
                active_corpus: Mutex::new(corpus_path),
                active_data_root: Mutex::new(dirs.data_dir().canonicalize()?),
                config_contents: Mutex::new(config_contents),
                engine: Mutex::new(engine),
                layouts: Mutex::new(layouts),
                generated: Mutex::new(Vec::new()),
                dirs,
                config: Mutex::new(config),
                cancel_flag: Arc::new(AtomicBool::new(false)),
                generating: Arc::new(AtomicBool::new(false)),
            });

            // File watcher: auto-reload when config.toml or layout files change.
            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let mut watch_target = watch_target;
                    let mut watch_layouts = watch_layouts;
                    let mut watcher =
                        match watch_paths(tx, &watch_config, &watch_target, &watch_layouts) {
                            Ok(watcher) => watcher,
                            Err(error) => {
                                eprintln!("File watcher init failed: {error}");
                                return;
                            }
                        };
                    let mut link_paths = Vec::new();
                    if let Err(error) = update_link_watches(
                        &mut watcher,
                        &watch_config,
                        &watch_target,
                        &watch_layouts,
                        &mut link_paths,
                    ) {
                        eprintln!("Config link watcher could not be registered: {error}");
                    }
                    while let Some(changes) = next_watch_changes(
                        &rx,
                        &watch_config,
                        &watch_target,
                        &watch_layouts,
                        &link_paths,
                        std::time::Duration::from_millis(250),
                    ) {
                        let state = app_handle.state::<AppState>();
                        if changes.config {
                            if let Err(error) = update_config_watch(
                                &mut watcher,
                                &watch_config,
                                &mut watch_target,
                                &watch_layouts,
                            ) {
                                eprintln!("Config target watcher could not be registered: {error}");
                            }
                            if let Err(error) = update_link_watches(
                                &mut watcher,
                                &watch_config,
                                &watch_target,
                                &watch_layouts,
                                &mut link_paths,
                            ) {
                                eprintln!("Config link watcher could not be registered: {error}");
                            }
                            if let Err(error) = update_layout_watch(
                                &mut watcher,
                                &layouts_path,
                                &mut watch_layouts,
                                &watch_config,
                                &watch_target,
                                &link_paths,
                            ) {
                                eprintln!("Layout watcher could not be updated: {error}");
                            }
                            if let Err(error) = reload_state(&state) {
                                eprintln!("Auto-reload failed: {error}");
                            }
                        }
                        // Keep layout changes even when config bytes are unchanged or invalid.
                        if changes.layouts {
                            match reload_layouts(&state) {
                                Ok(()) => {
                                    let _ = app_handle.emit("layouts-reloaded", ());
                                }
                                Err(error) => eprintln!("Layout reload failed: {error}"),
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_layouts,
            get_store_snapshot,
            list_languages,
            current_language,
            analyze_layout,
            analyze_custom,
            analyze_with_disabled,
            get_bigrams,
            get_trigrams,
            swap_keys,
            get_char_frequencies,
            set_language,
            reload_config,
            lookup_ngram,
            load_corpus,
            start_generate,
            save_generated,
            cancel_generate,
            get_layout_detail,
            save_layout_edit,
            fork_layout,
            delete_layout,
            save_custom_layout,
            get_session,
            set_session,
            get_config,
            set_config,
            get_defaults,
            list_weight_presets,
            save_weight_preset,
            load_weight_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod filename_boundary_tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "oxeylyzer-paths-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn lifecycle_state(root: &TestDir) -> AppState {
        let dirs = OxeylyzerDirs::with_override(root.0.clone());
        std::fs::create_dir_all(dirs.language_data_dir()).unwrap();
        for name in ["english", "dutch"] {
            let data = Data {
                name: name.into(),
                ..Data::default()
            };
            std::fs::write(
                corpus_path_for(&dirs.language_data_dir(), name),
                serde_json::to_vec(&data).unwrap(),
            )
            .unwrap();
        }
        let mut config = Config::with_defaults();
        config.layouts.clear();
        std::fs::write(
            dirs.config_file(),
            config_dto_to_toml(&config_to_dto(&config)),
        )
        .unwrap();
        let engine = Arc::new(Oxeylyzer::new(
            Data::load(dirs.data_dir().join(&config.corpus)).unwrap(),
            config.clone(),
        ));
        AppState {
            transition: Mutex::new(()),
            engine_revision: AtomicU64::new(0),
            engine_changed: Box::new(|_| {}),
            active_corpus: Mutex::new(dirs.data_dir().join(&config.corpus)),
            active_data_root: Mutex::new(dirs.data_dir().canonicalize().unwrap()),
            config_contents: Mutex::new(std::fs::read(dirs.config_file()).unwrap()),
            engine: Mutex::new(engine),
            layouts: Mutex::new(HashMap::new()),
            generated: Mutex::new(Vec::new()),
            dirs,
            config: Mutex::new(config),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            generating: Arc::new(AtomicBool::new(false)),
        }
    }

    fn watch_event(path: PathBuf) -> notify::Result<notify::Event> {
        Ok(
            notify::Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Any))
                .add_path(path),
        )
    }

    #[test]
    #[ignore = "requires native filesystem event delivery, unavailable in the managed execution environment"]
    fn watcher_native_atomic_replacement_keeps_monitoring() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let config = state.dirs.config_file().canonicalize().unwrap();
        let layouts = state.dirs.layouts_dir();
        std::fs::create_dir_all(&layouts).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let _watcher = watch_paths(tx, &config, &config, &layouts).unwrap();
        let probe = config.with_extension("probe");
        let ready_by = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            std::fs::write(&probe, b"readiness probe").unwrap();
            if rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_ok_and(|result| result.is_ok_and(|event| event.paths.contains(&probe)))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < ready_by,
                "native watcher never reported readiness probe"
            );
        }

        let wait_for_config = || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let event = rx
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .expect("native watcher did not report config change")
                    .unwrap();
                if event.paths.iter().any(|path| path == &config) {
                    break;
                }
            }
        };
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.weights.sfbs = 123.0;
        let temp = config.with_extension("tmp");
        std::fs::write(&temp, config_dto_to_toml(&dto)).unwrap();
        std::fs::rename(temp, &config).unwrap();
        wait_for_config();
        reload_state(&state).unwrap();
        assert_eq!(state.config.lock().unwrap().weights.sfbs, 123.0);
        // Drain replacement notifications before requiring a new write notification.
        while rx
            .recv_timeout(std::time::Duration::from_millis(400))
            .is_ok()
        {}
        dto.corpus = "static/language_data/dutch.json".into();
        std::fs::write(&config, config_dto_to_toml(&dto)).unwrap();
        wait_for_config();
        reload_state(&state).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    #[test]
    fn watcher_failed_reload_does_not_block_next_batch() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let config = state.dirs.config_file();
        let layouts = state.dirs.layouts_dir();
        let (tx, rx) = std::sync::mpsc::channel();
        std::fs::write(&config, "invalid = [").unwrap();
        tx.send(watch_event(config.clone())).unwrap();
        assert!(
            next_watch_changes(
                &rx,
                &config,
                &config,
                &layouts,
                &[],
                std::time::Duration::from_millis(5)
            )
            .unwrap()
            .config
        );
        assert!(reload_state(&state).is_err());
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.corpus = "static/language_data/dutch.json".into();
        std::fs::write(&config, config_dto_to_toml(&dto)).unwrap();
        tx.send(watch_event(config.clone())).unwrap();
        assert!(
            next_watch_changes(
                &rx,
                &config,
                &config,
                &layouts,
                &[],
                std::time::Duration::from_millis(5)
            )
            .unwrap()
            .config
        );
        reload_state(&state).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    #[test]
    fn watcher_rapid_edits_wait_for_trailing_write() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let config = state.dirs.config_file();
        let layouts = state.dirs.layouts_dir();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(config.clone())).unwrap();
        let second_tx = tx.clone();
        let second_path = config.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let sent = std::time::Instant::now();
            second_tx.send(watch_event(second_path)).unwrap();
            sent
        });
        let changes = next_watch_changes(
            &rx,
            &config,
            &config,
            &layouts,
            &[],
            std::time::Duration::from_millis(40),
        )
        .unwrap();
        assert!(changes.config);
        assert!(writer.join().unwrap().elapsed() >= std::time::Duration::from_millis(40));
    }

    #[test]
    fn watcher_remove_and_rename_preserve_categories() {
        let config = PathBuf::from("/config/config.toml");
        let layouts = PathBuf::from("/data/layouts");
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Ok(notify::Event::new(notify::EventKind::Remove(
            notify::event::RemoveKind::File,
        ))
        .add_path(layouts.join("english/gust.dof"))))
            .unwrap();
        tx.send(Ok(notify::Event::new(notify::EventKind::Modify(
            notify::event::ModifyKind::Name(notify::event::RenameMode::Both),
        ))
        .add_path(PathBuf::from("/config/new.tmp"))
        .add_path(config.clone())))
            .unwrap();
        let changes = next_watch_changes(
            &rx,
            &config,
            &config,
            &layouts,
            &[],
            std::time::Duration::from_millis(5),
        )
        .unwrap();
        assert_eq!(
            changes,
            WatchChanges {
                config: true,
                layouts: true
            }
        );
    }

    #[test]
    fn store_snapshot_keeps_engine_and_layouts_from_same_transition() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let mut layout = Layout::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../oxeylyzer-core/static/layouts/gust.dof"
        ))
        .unwrap();
        layout.name = "old".into();
        state.layouts.lock().unwrap().insert("gust".into(), layout);
        let (old_engine, old_layouts) = store_state_snapshot(&state);
        let transition = state.transition.lock().unwrap();
        let data = Data::load(corpus_path_for(&state.dirs.language_data_dir(), "dutch")).unwrap();
        let config = state.config.lock().unwrap().clone();
        *state.engine.lock().unwrap() = Arc::new(Oxeylyzer::new(data, config));
        state.layouts.lock().unwrap().get_mut("gust").unwrap().name = "new".into();
        drop(transition);
        let (new_engine, new_layouts) = store_state_snapshot(&state);
        assert_eq!(
            (old_engine.language.as_str(), old_layouts[0].name.as_str()),
            ("english", "old")
        );
        assert_eq!(
            (new_engine.language.as_str(), new_layouts[0].name.as_str()),
            ("dutch", "new")
        );
    }

    #[cfg(unix)]
    #[test]
    fn watcher_switches_target_when_config_symlink_changes() {
        use notify::Watcher;
        use std::os::unix::fs::symlink;
        #[derive(Default)]
        struct RecordingWatcher {
            added: Vec<PathBuf>,
            removed: Vec<PathBuf>,
        }
        impl notify::Watcher for RecordingWatcher {
            fn new<F: notify::EventHandler>(_: F, _: notify::Config) -> notify::Result<Self> {
                Ok(Self::default())
            }
            fn watch(&mut self, path: &Path, _: notify::RecursiveMode) -> notify::Result<()> {
                self.added.push(path.to_owned());
                Ok(())
            }
            fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
                self.removed.push(path.to_owned());
                Ok(())
            }
            fn kind() -> notify::WatcherKind {
                notify::WatcherKind::PollWatcher
            }
        }
        let root = TestDir::new();
        let first = root.0.join("first");
        let second = root.0.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("settings.toml"), "first").unwrap();
        std::fs::write(second.join("settings.toml"), "second").unwrap();
        let link = root.0.join("config.toml");
        symlink(first.join("settings.toml"), &link).unwrap();
        let mut target = link.canonicalize().unwrap();
        let mut watcher = RecordingWatcher::default();
        watcher
            .watch(&first, notify::RecursiveMode::NonRecursive)
            .unwrap();
        std::fs::remove_file(&link).unwrap();
        symlink(second.join("settings.toml"), &link).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(link.clone())).unwrap();
        assert!(
            next_watch_changes(
                &rx,
                &link,
                &target,
                &root.0.join("layouts"),
                &[],
                std::time::Duration::from_millis(5)
            )
            .unwrap()
            .config
        );
        update_config_watch(&mut watcher, &link, &mut target, &root.0.join("layouts")).unwrap();
        assert_eq!(target, second.join("settings.toml").canonicalize().unwrap());
        assert!(watcher.added.contains(&second.canonicalize().unwrap()));
        assert!(watcher.removed.contains(&first.canonicalize().unwrap()));
        tx.send(watch_event(target.clone())).unwrap();
        assert!(
            next_watch_changes(
                &rx,
                &link,
                &target,
                &root.0.join("layouts"),
                &[],
                std::time::Duration::from_millis(5)
            )
            .unwrap()
            .config
        );
    }

    #[cfg(unix)]
    #[test]
    fn watcher_detects_intermediate_directory_symlink_replacement() {
        use std::os::unix::fs::symlink;
        let root = TestDir::new();
        let base = root.0.canonicalize().unwrap();
        let first = base.join("first");
        let second = base.join("second");
        let links = base.join("links");
        for path in [&first, &second, &links] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(first.join("settings.toml"), "first").unwrap();
        std::fs::write(second.join("settings.toml"), "second").unwrap();
        let intermediate = links.join("current");
        symlink(&first, &intermediate).unwrap();
        let config = base.join("config.toml");
        symlink(intermediate.join("settings.toml"), &config).unwrap();
        let target = config.canonicalize().unwrap();
        let replacement = links.join("replacement");
        symlink(&second, &replacement).unwrap();
        std::fs::rename(replacement, &intermediate).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(intermediate)).unwrap();
        drop(tx);
        assert!(
            next_watch_changes(
                &rx,
                &config,
                &target,
                &root.0.join("layouts"),
                &[],
                std::time::Duration::from_millis(5),
            )
            .is_some_and(|changes| changes.config)
        );
    }

    #[cfg(unix)]
    #[test]
    fn watcher_detects_replaced_link_after_it_becomes_a_directory() {
        use std::os::unix::fs::symlink;

        let root = TestDir::new();
        let root = root.0.canonicalize().unwrap();
        let first = root.join("first");
        std::fs::create_dir(&first).unwrap();
        std::fs::write(first.join("settings.toml"), "first").unwrap();
        let intermediate = root.join("current");
        symlink(&first, &intermediate).unwrap();
        let config = root.join("config.toml");
        symlink(intermediate.join("settings.toml"), &config).unwrap();
        let target = config.canonicalize().unwrap();
        let known_links = config_link_paths(&config);
        std::fs::remove_file(&intermediate).unwrap();
        std::fs::create_dir(&intermediate).unwrap();
        std::fs::write(intermediate.join("settings.toml"), "second").unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(intermediate)).unwrap();
        drop(tx);

        assert!(
            next_watch_changes(
                &rx,
                &config,
                &target,
                &root.join("layouts"),
                &known_links,
                std::time::Duration::from_millis(5),
            )
            .is_some_and(|changes| changes.config)
        );
    }

    #[cfg(unix)]
    #[test]
    fn polling_watcher_follows_retargeted_data_directory() {
        use notify::Watcher;
        use std::os::unix::fs::symlink;
        use std::time::{Duration, Instant};

        let root = TestDir::new();
        let root = root.0.canonicalize().unwrap();
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::create_dir_all(first.join("static/layouts")).unwrap();
        std::fs::create_dir_all(second.join("static/layouts")).unwrap();
        std::fs::write(first.join("config.toml"), "first").unwrap();
        std::thread::sleep(Duration::from_secs(1));
        std::fs::write(second.join("config.toml"), "second").unwrap();
        let link = root.join("data");
        symlink(&first, &link).unwrap();
        let config = config_watch_path(&link.join("config.toml")).unwrap();
        assert!(config_link_paths(&config).contains(&link));

        let mut layouts = first.join("static/layouts");
        let mut target = config.canonicalize().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::PollWatcher::new(
            tx,
            notify::Config::default()
                .with_poll_interval(Duration::from_millis(30))
                .with_compare_contents(true),
        )
        .unwrap();
        watcher
            .watch(
                config.parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        watcher
            .watch(
                target.parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        watcher
            .watch(&layouts, notify::RecursiveMode::Recursive)
            .unwrap();
        let mut links_watched = Vec::new();
        update_link_watches(&mut watcher, &config, &target, &layouts, &mut links_watched).unwrap();

        let replacement = root.join("replacement");
        symlink(&second, &replacement).unwrap();
        std::fs::rename(replacement, &link).unwrap();
        let wait_for = |wanted: &Path| {
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let event = rx
                    .recv_timeout(end.saturating_duration_since(Instant::now()))
                    .expect("watcher did not report change");
                if let Ok(event) = event
                    && event.paths.iter().any(|path| path == wanted)
                {
                    break event;
                }
            }
        };
        let event = wait_for(&link);
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        event_tx.send(Ok(event)).unwrap();
        drop(event_tx);
        assert!(
            next_watch_changes(
                &event_rx,
                &config,
                &target,
                &layouts,
                &[],
                Duration::from_millis(5)
            )
            .unwrap()
            .config
        );
        update_config_watch(&mut watcher, &config, &mut target, &layouts).unwrap();
        update_link_watches(&mut watcher, &config, &target, &layouts, &mut links_watched).unwrap();
        update_layout_watch(
            &mut watcher,
            &link.join("static/layouts"),
            &mut layouts,
            &config,
            &target,
            &links_watched,
        )
        .unwrap();
        assert_eq!(target, second.join("config.toml"));
        std::fs::write(&target, "second changed").unwrap();
        wait_for(&target);
        let new_layout = second.join("static/layouts/new.dof");
        std::fs::write(&new_layout, "layout").unwrap();
        let event = wait_for(&new_layout);
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        event_tx.send(Ok(event)).unwrap();
        drop(event_tx);
        assert!(
            next_watch_changes(
                &event_rx,
                &config,
                &target,
                &layouts,
                &[],
                Duration::from_millis(5)
            )
            .unwrap()
            .layouts
        );
    }

    #[cfg(unix)]
    #[test]
    fn polling_watcher_follows_intermediate_directory_symlink() {
        use notify::Watcher;
        use std::os::unix::fs::symlink;
        use std::time::Duration;
        let root = TestDir::new();
        let root = root.0.canonicalize().unwrap();
        let first = root.join("first");
        let second = root.join("second");
        let links = root.join("links");
        for path in [&first, &second, &links] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(first.join("settings.toml"), "first").unwrap();
        // PollWatcher compares followed directory modification times in whole seconds.
        std::thread::sleep(Duration::from_secs(1));
        std::fs::write(second.join("settings.toml"), "second").unwrap();
        let intermediate = links.join("current");
        symlink(&first, &intermediate).unwrap();
        let config = root.join("config.toml");
        symlink(intermediate.join("settings.toml"), &config).unwrap();
        let layouts = root.join("layouts");
        let mut target = config.canonicalize().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::PollWatcher::new(
            tx,
            notify::Config::default()
                .with_poll_interval(Duration::from_millis(30))
                .with_compare_contents(true),
        )
        .unwrap();
        watcher
            .watch(
                config.parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        watcher
            .watch(
                target.parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        let mut links_watched = Vec::new();
        update_link_watches(&mut watcher, &config, &target, &layouts, &mut links_watched).unwrap();
        let replacement = links.join("replacement");
        symlink(&second, &replacement).unwrap();
        std::fs::rename(replacement, &intermediate).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let event = rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .expect("missing link event");
            if event.is_ok_and(|event| event.paths.contains(&intermediate)) {
                break;
            }
        }
        update_config_watch(&mut watcher, &config, &mut target, &layouts).unwrap();
        update_link_watches(&mut watcher, &config, &target, &layouts, &mut links_watched).unwrap();
        assert_eq!(target, second.join("settings.toml"));
        std::fs::write(&target, "second changed").unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let event = rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .expect("missing target event");
            if event.is_ok_and(|event| event.paths.contains(&target)) {
                break;
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn polling_watcher_detects_retarget_and_new_target_edits() {
        use notify::Watcher;
        use std::os::unix::fs::symlink;
        use std::time::{Duration, Instant};
        let root = TestDir::new();
        let first = root.0.join("first");
        let second = root.0.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("settings.toml"), "first").unwrap();
        std::fs::write(second.join("settings.toml"), "second").unwrap();
        let link = root.0.canonicalize().unwrap().join("config.toml");
        symlink(first.join("settings.toml"), &link).unwrap();
        let mut target = link.canonicalize().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::PollWatcher::new(
            tx,
            notify::Config::default()
                .with_poll_interval(Duration::from_millis(50))
                .with_compare_contents(true),
        )
        .unwrap();
        watcher
            .watch(link.parent().unwrap(), notify::RecursiveMode::NonRecursive)
            .unwrap();
        watcher
            .watch(
                target.parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        let replacement = link.with_extension("new");
        symlink(second.join("settings.toml"), &replacement).unwrap();
        std::fs::rename(replacement, &link).unwrap();
        let wait_for = |wanted: &Path| {
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let event = rx
                    .recv_timeout(end.saturating_duration_since(Instant::now()))
                    .expect("watcher did not report change");
                if event.is_ok_and(|event| event.paths.iter().any(|path| path == wanted)) {
                    break;
                }
            }
        };
        wait_for(&link);
        update_config_watch(&mut watcher, &link, &mut target, &root.0.join("layouts")).unwrap();
        std::fs::write(&target, "second changed").unwrap();
        wait_for(&target);
    }

    #[test]
    fn layout_snapshot_waits_for_engine_transition() {
        let root = TestDir::new();
        let state = Arc::new(lifecycle_state(&root));
        let layout = Layout::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../oxeylyzer-core/static/layouts/gust.dof"
        ))
        .unwrap();
        state.layouts.lock().unwrap().insert("gust".into(), layout);
        let transition = state.transition.lock().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let reader = {
            let state = state.clone();
            std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                let (engine, layout) = layout_snapshot(&state, "gust").unwrap();
                done_tx
                    .send((engine.language.clone(), layout.name))
                    .unwrap();
            })
        };
        started_rx.recv().unwrap();
        assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_millis(30))
                .is_err()
        );
        let corpus = corpus_path_for(&state.dirs.language_data_dir(), "dutch");
        let data = Data::load(&corpus).unwrap();
        let config = state.config.lock().unwrap().clone();
        *state.engine.lock().unwrap() = Arc::new(Oxeylyzer::new(data, config));
        state.layouts.lock().unwrap().get_mut("gust").unwrap().name = "new layout".into();
        drop(transition);
        assert_eq!(
            done_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            ("dutch".into(), "new layout".into())
        );
        reader.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn config_save_follows_symlink_and_preserves_it() {
        use std::os::unix::fs::symlink;
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let config = state.dirs.config_file();
        let target = root.0.join("actual-config.toml");
        std::fs::rename(&config, &target).unwrap();
        symlink(&target, &config).unwrap();
        let snapshot = get_config_state(&state);
        let mut changed = snapshot.config;
        changed.weights.sfbs = 123.0;
        save_config_state(&state, changed, &snapshot.revision).unwrap();
        assert!(
            std::fs::symlink_metadata(&config)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let saved: Config = toml::from_str(&std::fs::read_to_string(target).unwrap()).unwrap();
        assert_eq!(saved.weights.sfbs, 123.0);
    }

    #[test]
    fn watcher_layout_reload_preserves_current_engine_and_generation() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let engine = state.engine.lock().unwrap().clone();
        state
            .generated
            .lock()
            .unwrap()
            .push(generation_result(&engine));
        reload_layouts(&state).unwrap();
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert_eq!(state.generated.lock().unwrap().len(), 1);
        assert!(!state.cancel_flag.load(Ordering::Relaxed));
    }

    #[cfg(unix)]
    #[test]
    fn layout_reload_uses_active_root_after_failed_data_retarget() {
        use std::os::unix::fs::symlink;

        let root = TestDir::new();
        let first = TestDir::new();
        let second = TestDir::new();
        let mut state = lifecycle_state(&first);
        let _second_state = lifecycle_state(&second);
        let mut layout = Layout::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../oxeylyzer-core/static/layouts/gust.dof"
        ))
        .unwrap();
        for (dir, name) in [(&first, "first"), (&second, "second")] {
            let layouts = dir.0.join("static/layouts");
            std::fs::create_dir_all(&layouts).unwrap();
            layout.name = name.into();
            std::fs::write(
                layouts.join("layout.dof"),
                serde_json::to_vec(&layout).unwrap(),
            )
            .unwrap();
        }
        let config = state.config.lock().unwrap().clone();
        state.config.lock().unwrap().layouts = vec!["static/layouts/*.dof".into()];
        let link = root.0.join("data");
        symlink(&first.0, &link).unwrap();
        state.dirs = OxeylyzerDirs::with_override(link.clone());
        std::fs::write(second.0.join("config.toml"), "invalid = [").unwrap();
        let replacement = root.0.join("replacement");
        symlink(&second.0, &replacement).unwrap();
        std::fs::rename(replacement, &link).unwrap();

        assert!(reload_state(&state).is_err());
        reload_layouts(&state).unwrap();
        assert!(state.layouts.lock().unwrap().contains_key("first"));
        assert!(!state.layouts.lock().unwrap().contains_key("second"));
        assert_eq!(state.engine.lock().unwrap().language, "english");
        assert_eq!(config.corpus, state.config.lock().unwrap().corpus);
    }

    #[test]
    fn watcher_mixed_categories_survive_one_burst() {
        let config = PathBuf::from("/config/config.toml");
        let layouts = PathBuf::from("/data/layouts");
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(config.clone())).unwrap();
        tx.send(watch_event(layouts.join("english/gust.dof")))
            .unwrap();
        let changes = next_watch_changes(
            &rx,
            &config,
            &config,
            &layouts,
            &[],
            std::time::Duration::from_millis(10),
        )
        .unwrap();
        assert_eq!(
            changes,
            WatchChanges {
                config: true,
                layouts: true
            }
        );
    }

    #[test]
    fn watcher_unrelated_parent_events_are_ignored() {
        let config = PathBuf::from("/config/config.toml");
        let layouts = PathBuf::from("/data/layouts");
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(PathBuf::from("/config/unrelated.toml")))
            .unwrap();
        drop(tx);
        assert_eq!(
            next_watch_changes(
                &rx,
                &config,
                &config,
                &layouts,
                &[],
                std::time::Duration::from_millis(10)
            ),
            None
        );
    }

    #[test]
    fn watcher_invalid_then_valid_burst_loads_latest() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let config = state.dirs.config_file();
        let layouts = state.dirs.layouts_dir();
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.corpus = "static/language_data/dutch.json".into();
        std::fs::write(&config, "invalid = [").unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(watch_event(config.clone())).unwrap();
        let write_path = config.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            std::fs::write(&write_path, config_dto_to_toml(&dto)).unwrap();
            tx.send(watch_event(write_path)).unwrap();
        });
        let changes = next_watch_changes(
            &rx,
            &config,
            &config,
            &layouts,
            &[],
            std::time::Duration::from_millis(60),
        )
        .unwrap();
        assert!(changes.config);
        let result = reload_state(&state);
        writer.join().unwrap();
        result.unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    fn generation_result(engine: &Oxeylyzer) -> FastLayout {
        let layout = Layout::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../oxeylyzer-core/static/layouts/gust.dof"
        ))
        .unwrap();
        engine.fast_layout(&layout, &[])
    }

    #[test]
    fn lifecycle_engine_invalidation_signals_only_actual_transitions() {
        let root = TestDir::new();
        let mut state = lifecycle_state(&root);
        let revisions = Arc::new(Mutex::new(Vec::new()));
        let state_ref = Arc::new(Mutex::new(None::<std::sync::Weak<AppState>>));
        state.engine_changed = Box::new({
            let revisions = revisions.clone();
            let state_ref = state_ref.clone();
            move |revision| {
                let state = state_ref
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap();
                assert!(state.transition.try_lock().is_err());
                assert!(state.generated.lock().unwrap().is_empty());
                assert_eq!(state.engine_revision.load(Ordering::Relaxed), revision);
                revisions.lock().unwrap().push(revision);
            }
        });
        let state = Arc::new(state);
        *state_ref.lock().unwrap() = Some(Arc::downgrade(&state));
        set_language_state(&state, "dutch").unwrap();
        assert_eq!(*revisions.lock().unwrap(), vec![1]);
        let snapshot = get_config_state(&state);
        save_config_state(&state, snapshot.config, &snapshot.revision).unwrap();
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2]);
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.weights.sfbs = 123.0;
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2, 3]);

        let engine = state.engine.lock().unwrap().clone();
        let result = generation_result(&engine);
        let dto = layout_to_dto(&engine, &result.clone().into());
        finish_generation(
            &state,
            &engine,
            3,
            vec![result],
            vec![dto],
            false,
            |event| {
                assert_eq!(event["revision"], 3);
                assert_eq!(event["results"].as_array().unwrap().len(), 1);
                assert_eq!(*revisions.lock().unwrap(), vec![1, 2, 3]);
            },
        );
        // A delayed self-save notification must not invalidate the new results.
        reload_state(&state).unwrap();
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2, 3]);
        assert_eq!(state.generated.lock().unwrap().len(), 1);
        std::fs::write(state.dirs.config_file(), "invalid = [").unwrap();
        assert!(reload_state(&state).is_err());
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2, 3]);
        set_language_state(&state, "english").unwrap();
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2, 3, 4]);
        assert!(state.generated.lock().unwrap().is_empty());
    }

    #[test]
    fn lifecycle_generation_replacement_before_completion_discards_results() {
        let root = TestDir::new();
        let state = Arc::new(lifecycle_state(&root));
        let engine = state.engine.lock().unwrap().clone();
        let result = generation_result(&engine);
        let dto = layout_to_dto(&engine, &result.clone().into());
        state.generating.store(true, Ordering::SeqCst);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let worker_state = state.clone();
        let worker = std::thread::spawn(move || {
            ready_tx.send(()).unwrap();
            finish_rx.recv().unwrap();
            let mut event = None;
            finish_generation(
                &worker_state,
                &engine,
                0,
                vec![result],
                vec![dto],
                false,
                |payload| event = Some(payload),
            );
            event.unwrap()
        });
        ready_rx.recv().unwrap();
        set_language_state(&state, "dutch").unwrap();
        assert!(state.cancel_flag.load(Ordering::Relaxed));
        assert!(state.generating.load(Ordering::SeqCst));
        finish_tx.send(()).unwrap();
        let event = worker.join().unwrap();
        assert!(
            state.generated.lock().unwrap().is_empty(),
            "stale results became saveable"
        );
        assert_eq!(event["results"], serde_json::json!([]));
        assert_eq!(event["cancelled"], true);
        assert_eq!(event["revision"], 0);
        assert!(event["reason"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(!state.generating.load(Ordering::SeqCst));
    }

    #[test]
    fn lifecycle_generation_ordinary_cancel_keeps_partial_results() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let engine = state.engine.lock().unwrap().clone();
        let result = generation_result(&engine);
        let dto = layout_to_dto(&engine, &result.clone().into());
        let expected_dto = serde_json::to_value(&dto).unwrap();
        state.cancel_flag.store(true, Ordering::Relaxed);
        state.generating.store(true, Ordering::SeqCst);
        let mut event = None;
        finish_generation(
            &state,
            &engine,
            0,
            vec![result],
            vec![dto],
            true,
            |payload| {
                assert!(
                    state.transition.try_lock().is_err(),
                    "publication must exclude engine replacement"
                );
                assert!(!state.generating.load(Ordering::SeqCst));
                event = Some(payload);
            },
        );
        assert_eq!(state.generated.lock().unwrap().len(), 1);
        let event = event.unwrap();
        assert_eq!(event["results"], serde_json::json!([expected_dto]));
        assert_eq!(event["cancelled"], true);
        assert!(event["reason"].is_null());
        assert!(!state.generating.load(Ordering::SeqCst));
    }

    #[test]
    fn lifecycle_switch_then_save_preserves_session_corpus() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        set_language_state(&state, "dutch").unwrap();
        let snapshot = get_config_state(&state);
        let mut dto = snapshot.config;
        dto.weights.sfbs = 123.0;
        save_config_state(&state, dto, &snapshot.revision).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
        let saved = Config::with_loaded_weights(state.dirs.config_file()).unwrap();
        assert!(saved.corpus.ends_with("english.json"));
    }

    #[test]
    fn lifecycle_external_unchanged_corpus_preserves_session_selection() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        set_language_state(&state, "dutch").unwrap();
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.weights.sfbs = 123.0;
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
        assert_eq!(state.config.lock().unwrap().weights.sfbs, 123.0);
    }

    #[cfg(unix)]
    #[test]
    fn lifecycle_reloads_retargeted_data_directory_with_unchanged_config() {
        use std::os::unix::fs::symlink;

        let root = TestDir::new();
        let first = TestDir::new();
        let second = TestDir::new();
        let mut state = lifecycle_state(&first);
        let _second_state = lifecycle_state(&second);
        let data = Data {
            name: "replacement".into(),
            ..Data::default()
        };
        std::fs::write(
            corpus_path_for(&second.0.join("static/language_data"), "english"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        let link = root.0.join("data");
        symlink(&first.0, &link).unwrap();
        state.dirs = OxeylyzerDirs::with_override(link.clone());
        *state.active_corpus.lock().unwrap() = link.join(&state.config.lock().unwrap().corpus);

        let replacement = root.0.join("replacement");
        symlink(&second.0, &replacement).unwrap();
        std::fs::rename(replacement, link).unwrap();
        reload_state(&state).unwrap();

        assert_eq!(state.engine.lock().unwrap().language, "replacement");
    }

    #[test]
    fn lifecycle_external_changed_corpus_selects_new_language() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        dto.corpus = "static/language_data/dutch.json".into();
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    #[test]
    fn config_save_round_trips_control_characters_in_paths() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let snapshot = get_config_state(&state);
        let mut config = snapshot.config;
        config.layouts = vec!["layouts/with\u{7f}character".into()];
        config.corpus_configs = "rules/with\u{1}character".into();
        save_config_state(&state, config, &snapshot.revision).unwrap();
        let saved = Config::with_loaded_weights(state.dirs.config_file()).unwrap();
        assert_eq!(
            saved.layouts,
            vec![PathBuf::from("layouts/with\u{7f}character")]
        );
        assert_eq!(
            saved.corpus_configs,
            PathBuf::from("rules/with\u{1}character")
        );
    }

    #[test]
    fn lifecycle_failed_preparation_preserves_file_and_engine() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let original = std::fs::read(state.dirs.config_file()).unwrap();
        let engine = state.engine.lock().unwrap().clone();
        let snapshot = get_config_state(&state);
        let mut dto = snapshot.config;
        dto.corpus = "missing.json".into();
        assert!(save_config_state(&state, dto, &snapshot.revision).is_err());
        assert_eq!(std::fs::read(state.dirs.config_file()).unwrap(), original);
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
    }

    #[test]
    fn config_save_preserves_state_when_a_matched_layout_is_invalid() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let layouts = state.dirs.layouts_dir();
        std::fs::create_dir_all(&layouts).unwrap();
        let path = layouts.join("gust.dof");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../oxeylyzer-core/static/layouts/gust.dof"
            ),
            &path,
        )
        .unwrap();
        let mut dto = get_config_state(&state).config;
        dto.layouts = vec!["static/layouts/*.dof".into()];
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        assert!(state.layouts.lock().unwrap().contains_key("gust"));
        let engine = state.engine.lock().unwrap().clone();
        let original = std::fs::read(state.dirs.config_file()).unwrap();
        std::fs::write(path, "invalid layout").unwrap();
        assert!(reload_layouts(&state).is_err());
        assert!(state.layouts.lock().unwrap().contains_key("gust"));
        let snapshot = get_config_state(&state);
        dto = snapshot.config;
        dto.weights.sfbs = 123.0;

        assert!(save_config_state(&state, dto, &snapshot.revision).is_err());
        assert_eq!(std::fs::read(state.dirs.config_file()).unwrap(), original);
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert!(state.layouts.lock().unwrap().contains_key("gust"));
    }

    #[test]
    fn watcher_layout_repair_retries_pending_config() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let layouts = state.dirs.layouts_dir();
        std::fs::create_dir_all(&layouts).unwrap();
        let path = layouts.join("gust.dof");
        let valid_layout = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../oxeylyzer-core/static/layouts/gust.dof"
        ));
        std::fs::write(&path, valid_layout).unwrap();
        let mut dto = get_config_state(&state).config;
        dto.layouts = vec!["static/layouts/*.dof".into()];
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        let engine = state.engine.lock().unwrap().clone();

        std::fs::write(&path, "invalid layout").unwrap();
        dto.weights.sfbs = 123.0;
        let contents = config_dto_to_toml(&dto);
        std::fs::write(state.dirs.config_file(), &contents).unwrap();
        assert!(reload_state(&state).is_err());
        assert!(reload_layouts(&state).is_err());
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));

        std::fs::write(&path, valid_layout).unwrap();
        reload_layouts(&state).unwrap();

        assert_eq!(state.config.lock().unwrap().weights.sfbs, 123.0);
        assert!(!Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert_eq!(*state.config_contents.lock().unwrap(), contents.as_bytes());
        assert!(state.layouts.lock().unwrap().contains_key("gust"));
    }

    #[test]
    fn lifecycle_self_save_event_keeps_the_published_engine() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        set_language_state(&state, "dutch").unwrap();
        let snapshot = get_config_state(&state);
        save_config_state(&state, snapshot.config, &snapshot.revision).unwrap();
        let engine = state.engine.lock().unwrap().clone();
        reload_state(&state).unwrap();
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    #[test]
    fn lifecycle_external_conflict_preserves_file_engine_and_cleans_temporary_file() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let engine = state.engine.lock().unwrap().clone();
        let snapshot = get_config_state(&state);
        let external = b"an external edit not yet reloaded";
        std::fs::write(state.dirs.config_file(), external).unwrap();
        assert!(
            save_config_state(&state, snapshot.config, &snapshot.revision)
                .unwrap_err()
                .contains("changed externally")
        );
        assert_eq!(std::fs::read(state.dirs.config_file()).unwrap(), external);
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert!(
            !std::fs::read_dir(&root.0)
                .unwrap()
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        );
    }

    #[test]
    fn lifecycle_stale_form_after_external_reload_preserves_file_and_active_state() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let snapshot = get_config_state(&state);
        let mut stale = snapshot.config;
        stale.weights.sfbs = 123.0;
        let mut external = config_to_dto(&state.config.lock().unwrap());
        external.corpus = "static/language_data/dutch.json".into();
        external.weights.sfbs = 456.0;
        let external_contents = config_dto_to_toml(&external).into_bytes();
        std::fs::write(state.dirs.config_file(), &external_contents).unwrap();
        reload_state(&state).unwrap();
        let engine = state.engine.lock().unwrap().clone();
        let active_corpus = state.active_corpus.lock().unwrap().clone();

        assert!(save_config_state(&state, stale, &snapshot.revision).is_err());

        assert_eq!(
            std::fs::read(state.dirs.config_file()).unwrap(),
            external_contents
        );
        assert_eq!(*state.config_contents.lock().unwrap(), external_contents);
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        assert_eq!(*state.active_corpus.lock().unwrap(), active_corpus);
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
        assert_eq!(state.config.lock().unwrap().weights.sfbs, 456.0);
    }

    #[test]
    fn lifecycle_saved_revision_accepts_successive_form_edits() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let snapshot = get_config_state(&state);
        let mut form = snapshot.config;
        form.weights.sfbs = 123.0;
        let saved_revision = save_config_state(&state, form.clone(), &snapshot.revision).unwrap();
        assert_ne!(saved_revision, snapshot.revision);
        assert_eq!(saved_revision, get_config_state(&state).revision);

        form.weights.sfbs = 456.0;
        let latest_revision = save_config_state(&state, form, &saved_revision).unwrap();

        assert_eq!(
            std::fs::read(state.dirs.config_file()).unwrap(),
            latest_revision
        );
        assert_eq!(get_config_state(&state).revision, latest_revision);
        assert_eq!(state.config.lock().unwrap().weights.sfbs, 456.0);
        assert_eq!(
            Config::with_loaded_weights(state.dirs.config_file())
                .unwrap()
                .weights
                .sfbs,
            456.0
        );
    }

    #[test]
    fn lifecycle_invalid_external_write_can_be_followed_by_valid_write() {
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let engine = state.engine.lock().unwrap().clone();
        let mut dto = config_to_dto(&state.config.lock().unwrap());
        std::fs::write(state.dirs.config_file(), "invalid = [").unwrap();
        assert!(reload_state(&state).is_err());
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
        dto.corpus = "static/language_data/dutch.json".into();
        std::fs::write(state.dirs.config_file(), config_dto_to_toml(&dto)).unwrap();
        reload_state(&state).unwrap();
        assert_eq!(state.engine.lock().unwrap().language, "dutch");
    }

    #[cfg(unix)]
    #[test]
    fn config_save_rejects_read_only_file() {
        use std::os::unix::fs::PermissionsExt;
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let path = state.dirs.config_file();
        let original = std::fs::read(&path).unwrap();
        let engine = state.engine.lock().unwrap().clone();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        if std::fs::OpenOptions::new().write(true).open(&path).is_ok() {
            return;
        }
        let snapshot = get_config_state(&state);
        assert!(save_config_state(&state, snapshot.config, &snapshot.revision).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(Arc::ptr_eq(&engine, &state.engine.lock().unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn lifecycle_atomic_replace_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let root = TestDir::new();
        let state = lifecycle_state(&root);
        let path = state.dirs.config_file();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let snapshot = get_config_state(&state);
        save_config_state(&state, snapshot.config, &snapshot.revision).unwrap();
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn preset_save_rejects_traversal_without_overwriting_config() {
        let root = TestDir::new();
        let dirs = OxeylyzerDirs::with_override(root.0.clone());
        let sentinel = dirs.config_file();
        std::fs::write(&sentinel, b"original\0config").unwrap();
        let weights = config_to_dto(&Config::with_defaults()).weights;

        assert!(write_weight_preset(&dirs.weight_presets_dir(), "../config", &weights).is_err());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"original\0config");
    }

    #[test]
    fn preset_load_rejects_traversal_to_valid_weights() {
        let root = TestDir::new();
        let dirs = OxeylyzerDirs::with_override(root.0.clone());
        let presets = dirs.weight_presets_dir();
        std::fs::create_dir_all(&presets).unwrap();
        let weights = config_to_dto(&Config::with_defaults()).weights;
        std::fs::write(dirs.config_file(), weights_dto_to_toml(&weights)).unwrap();

        assert!(read_weight_preset(&presets, "../config").is_err());
    }

    #[test]
    fn valid_preset_names_round_trip() {
        let root = TestDir::new();
        let presets = root.0.join("presets");
        let mut weights = config_to_dto(&Config::with_defaults()).weights;
        weights.sfbs = 123.0;

        write_weight_preset(&presets, "My Weights", &weights).unwrap();
        assert!(presets.join("My Weights.toml").is_file());
        let loaded = read_weight_preset(&presets, "My Weights").unwrap();
        assert_eq!(
            serde_json::to_value(loaded).unwrap(),
            serde_json::to_value(weights).unwrap()
        );
    }

    #[test]
    fn layout_delete_rejects_traversal_without_removing_neighbor() {
        let root = TestDir::new();
        let dirs = OxeylyzerDirs::with_override(root.0.clone());
        let layouts = dirs.layouts_dir().join("english");
        std::fs::create_dir_all(&layouts).unwrap();
        let sentinel = dirs.layouts_dir().join("config.dof");
        std::fs::write(&sentinel, b"original\0layout").unwrap();

        assert!(delete_layout_file(&dirs.layouts_dir(), "english", "../config").is_err());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"original\0layout");
    }

    #[test]
    fn layout_delete_rejects_corpus_language_paths() {
        let root = TestDir::new();
        let dirs = OxeylyzerDirs::with_override(root.0.clone());
        std::fs::create_dir_all(dirs.layouts_dir()).unwrap();
        let outside = root.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        for (language, neighbor) in [
            ("..".to_string(), root.0.join("static/config.dof")),
            (
                outside.to_str().unwrap().to_string(),
                outside.join("config.dof"),
            ),
        ] {
            std::fs::write(&neighbor, b"original\0layout").unwrap();
            let engine = Oxeylyzer::new(
                Data {
                    name: language,
                    ..Data::default()
                },
                Config::with_defaults(),
            );

            assert!(
                delete_layout_file(&dirs.layouts_dir(), &engine.language, "config").is_err(),
                "accepted corpus language {:?}",
                engine.language
            );
            assert_eq!(std::fs::read(neighbor).unwrap(), b"original\0layout");
        }
    }

    #[test]
    fn layout_delete_uses_existing_name_normalization() {
        let root = TestDir::new();
        let layout = root.0.join("english/my_layout.dof");
        generated_layout::write_new(&layout, "layout").unwrap();

        delete_layout_file(&root.0, "english", "My Layout").unwrap();
        assert!(!layout.exists());
    }

    #[test]
    fn rejects_paths_and_accepts_names() {
        for name in [
            "",
            ".",
            "..",
            "../config",
            "/tmp/file",
            "a/b",
            "a\\b",
            "C:config",
        ] {
            assert!(validate_filename(name).is_err(), "accepted {name:?}");
        }
        for name in ["My Layout", "layout_1", "é"] {
            assert!(validate_filename(name).is_ok(), "rejected {name:?}");
        }
    }
}
