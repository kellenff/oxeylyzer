use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use oxeylyzer_core::{
    data::Data,
    fast_layout::{BigramPair, FastLayout, auto_name},
    generate::{LayoutStats, Oxeylyzer},
    layout::{Layout, LayoutMetadata},
    rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    weights::{Config, FingerWeights, MaxFingerUse, Weights},
};
use oxeylyzer_resources::OxeylyzerDirs;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

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

// ─── App State ────────────────────────────────────────────────────────────────

pub struct AppState {
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

fn load_all_layouts(config: &Config, base_path: &Path) -> HashMap<String, Layout> {
    config
        .layouts
        .iter()
        .flat_map(|p| {
            let full = base_path.join(p);
            let pattern = full.to_string_lossy().into_owned();
            glob::glob(&pattern)
                .into_iter()
                .flatten()
                .flatten()
                .flat_map(|path| {
                    Layout::load(&path).inspect_err(|e| {
                        eprintln!("Error loading layout '{}': {e}", path.display())
                    })
                })
                .map(|l| (l.name.to_lowercase(), l))
        })
        .collect()
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

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
fn list_layouts(state: tauri::State<'_, AppState>) -> Result<Vec<LayoutDto>, String> {
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let mut dtos: Vec<LayoutDto> = layouts
        .values()
        .map(|l| layout_to_dto(&engine, l))
        .collect();
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
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    Ok(layout_to_dto(&engine, layout))
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
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    let fl = custom_fast_layout(
        &engine,
        layout,
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

    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    let fl = custom_fast_layout(
        &engine,
        layout,
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
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    let mut fl = engine.fast_layout(layout, &[]);

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
        board: get_board_str(layout),
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
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    let fl = custom_fast_layout(&engine, layout, Some(&keys), &disabled_indices)?;

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
        board: get_board_str(layout),
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
    let engine = state.engine.lock().unwrap().clone();
    let layouts = state.layouts.lock().unwrap();
    let layout = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?;
    let original_keys = engine.fast_layout(layout, &[]).layout_str();
    let fl = custom_fast_layout(&engine, layout, None, &disabled_indices)?;
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
        board: get_board_str(layout),
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
    let total = engine.data.char_total as f64;
    if total == 0.0 {
        return Ok(vec![]);
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
    Ok(freqs)
}

#[tauri::command]
fn set_language(language: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let corpus_path = corpus_path_for(&state.dirs.language_data_dir(), &language);
    let data = Data::load(&corpus_path)
        .map_err(|e| format!("Failed to load corpus for '{language}': {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let new_layouts = load_all_layouts(&config, state.dirs.data_dir());

    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    state.generated.lock().unwrap().clear();
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
        *state.generated.lock().unwrap() = scored.into_iter().map(|(_, fl)| fl).collect();

        let _ = app.emit(
            "generate-done",
            serde_json::json!({ "results": layout_dtos, "cancelled": cancelled }),
        );

        generating.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
fn save_generated(
    index: usize,
    name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let mut generated = state.generated.lock().unwrap();

    let len = generated.len();
    let fl = generated
        .get_mut(index)
        .ok_or_else(|| format!("Index {index} out of bounds ({len} results)"))?;

    // Assign the chosen name.
    let save_name = name.unwrap_or_else(|| auto_name(&fl.keys, &fl.shape, &engine.mapping));
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
    let file_name = save_name.replace(' ', "_").to_lowercase();
    let path = state
        .dirs
        .layouts_dir()
        .join(&lang)
        .join(&file_name)
        .with_extension("dof");

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &json).map_err(|e| format!("Write failed: {e}"))?;

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
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn delete_layout(name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let lang = state.engine.lock().unwrap().language.clone();
    let file_name = name.replace(' ', "_").to_lowercase();
    let path = state
        .dirs
        .layouts_dir()
        .join(&lang)
        .join(&file_name)
        .with_extension("dof");

    if !path.exists() {
        return Err(format!(
            "Layout file '{}' not found — it may live outside the managed layouts directory.",
            path.display()
        ));
    }
    std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))?;
    state.layouts.lock().unwrap().remove(&name.to_lowercase());
    Ok(())
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
    let file_name = new_name.replace(' ', "_").to_lowercase();
    let path = state
        .dirs
        .layouts_dir()
        .join(&lang)
        .join(&file_name)
        .with_extension("dof");

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
    let layout: Layout = serde_json::from_value(dof_json.clone())
        .map_err(|e| format!("Invalid layout JSON: {e}"))?;
    let new_name = layout.name.clone();

    let lang = state.engine.lock().unwrap().language.clone();
    let original_path = layout_path(&state, &lang, &original_name);
    let new_path = layout_path(&state, &lang, &new_name);
    let is_rename = original_path != new_path;

    if is_rename {
        // Refuse rather than clobber, matching how the REPL handles a name collision.
        if new_path.exists() {
            return Err(format!(
                "Cannot rename to \"{new_name}\": a layout with that name already exists."
            ));
        }
        if !original_path.exists() {
            return Err(format!(
                "Cannot save: the file for \"{original_name}\" no longer exists on disk."
            ));
        }
    }

    let json = serde_json::to_string_pretty(&dof_json).map_err(|e| e.to_string())?;

    // Write first, then remove the old file. A failed write leaves the original
    // intact; delete-then-write would risk losing the layout.
    std::fs::write(&new_path, &json).map_err(|e| format!("Write failed: {e}"))?;
    if is_rename {
        std::fs::remove_file(&original_path)
            .map_err(|e| format!("Saved as \"{new_name}\" but could not remove the old file: {e}"))?;
    }

    let layout_loaded = Layout::load(&new_path).map_err(|e| e.to_string())?;
    let mut layouts = state.layouts.lock().unwrap();
    layouts.remove(&original_name.to_lowercase());
    layouts.insert(new_name.to_lowercase(), layout_loaded);
    Ok(())
}

/// Path of a layout's file within its language directory.
fn layout_path(state: &AppState, lang: &str, name: &str) -> PathBuf {
    let file_name = name.replace(' ', "_").to_lowercase();
    state
        .dirs
        .layouts_dir()
        .join(lang)
        .join(&file_name)
        .with_extension("dof")
}

#[tauri::command]
fn fork_layout(
    name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let lang = engine.language.clone();

    let mut layouts = state.layouts.lock().unwrap();
    let original = layouts
        .get(&name.to_lowercase())
        .ok_or_else(|| format!("Layout '{name}' not found"))?
        .clone();

    let mut forked = original.clone();
    forked.name = new_name.clone();

    let file_name = new_name.replace(' ', "_").to_lowercase();
    let path = state
        .dirs
        .layouts_dir()
        .join(&lang)
        .join(&file_name)
        .with_extension("dof");

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
fn get_config(state: tauri::State<'_, AppState>) -> Result<ConfigDto, String> {
    let config = state.config.lock().unwrap();
    Ok(config_to_dto(&config))
}

#[tauri::command]
fn set_config(config_dto: ConfigDto, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let new_weights = dto_to_weights(&config_dto.weights);
    let new_config = Config {
        corpus: PathBuf::from(&config_dto.corpus),
        layouts: config_dto.layouts.iter().map(PathBuf::from).collect(),
        corpus_configs: PathBuf::from(&config_dto.corpus_configs),
        trigram_precision: config_dto.trigram_precision,
        max_cores: config_dto.max_cores,
        weights: new_weights,
    };

    // Write config.toml as hand-built TOML string.
    let config_path = state.dirs.config_file();
    std::fs::write(&config_path, config_dto_to_toml(&config_dto))
        .map_err(|e| format!("Write failed: {e}"))?;

    // Rebuild engine with new config
    let corpus_path = state.dirs.data_dir().join(&new_config.corpus);
    let data = Data::load(&corpus_path).map_err(|e| format!("Failed to load corpus: {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, new_config.clone()));
    let new_layouts = load_all_layouts(&new_config, state.dirs.data_dir());

    *state.config.lock().unwrap() = new_config;
    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    // Generated layouts are tied to the old engine's character mapping.
    state.generated.lock().unwrap().clear();
    Ok(())
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
    let dir = preset_dir(&state.dirs);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&name).with_extension("toml");
    std::fs::write(&path, weights_dto_to_toml(&weights)).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_weight_preset(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<WeightsDto, String> {
    let path = preset_dir(&state.dirs).join(&name).with_extension("toml");
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    toml::from_str::<WeightsDto>(&s).map_err(|e| format!("Failed to parse preset '{name}': {e}"))
}

// ─── Reload Helper ────────────────────────────────────────────────────────────

fn reload_state(state: &AppState) -> Result<(), String> {
    let config = Config::with_loaded_weights(state.dirs.config_file())
        .map_err(|e| format!("Failed to reload config: {e}"))?;
    let corpus_path = state.dirs.data_dir().join(&config.corpus);
    let data = Data::load(&corpus_path).map_err(|e| format!("Failed to load corpus: {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let new_layouts = load_all_layouts(&config, state.dirs.data_dir());
    *state.config.lock().unwrap() = config;
    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    state.generated.lock().unwrap().clear();
    Ok(())
}

// ─── Entry Point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Startup status the frontend reads after it mounts.
///
/// The setup hook runs before the webview loads, so an event emitted there would be
/// missed by a listener that registers on mount. The last progress payload and any
/// failure message are recorded here so the UI can query them instead.
#[derive(Default)]
struct StartupStatus {
    error: Option<String>,
    progress: Option<serde_json::Value>,
}

#[tauri::command]
fn startup_error(status: tauri::State<'_, StartupStatus>) -> Option<String> {
    status.error.clone()
}

#[tauri::command]
fn download_progress(status: tauri::State<'_, StartupStatus>) -> Option<serde_json::Value> {
    status.progress.clone()
}

/// Resolves the data and config directories, honouring the dev override.
///
/// Returns a message naming the directory that could not be used, rather than
/// terminating: a user who double-clicked the app has no terminal to read a panic.
fn resolve_dirs() -> Result<OxeylyzerDirs, String> {
    match std::env::var("OXEYLYZER_DATA_DIR") {
        Ok(p) => Ok(OxeylyzerDirs::with_override(PathBuf::from(p))),
        Err(_) => OxeylyzerDirs::resolve().map_err(|e| {
            format!(
                "Could not set up the oxeylyzer data directory. Oxeylyzer keeps its \
                 layouts and language data in your user data directory. {e}"
            )
        }),
    }
}

/// Startup state plus the values the file watcher needs.
struct Initialized {
    state: AppState,
}

/// Performs the startup work that must succeed before the app is usable.
///
/// On first run the data files are downloaded synchronously before the engine is
/// built — the corpus and layouts must exist before they can be read. Each failure
/// returns a message naming both the step and the location involved, so the user can
/// tell what to change rather than just that something broke.
fn initialize(
    dirs: OxeylyzerDirs,
    app_handle: AppHandle,
    status: Arc<Mutex<StartupStatus>>,
) -> Result<Initialized, String> {
    if dirs.needs_data() {
        use oxeylyzer_resources::DownloadProgress;
        // ensure_data's progress callback must be 'static, so move the handle in.
        let emit_handle = app_handle.clone();
        let status = Arc::clone(&status);
        dirs.ensure_data(move |p| {
            let payload = match &p {
                DownloadProgress::Connecting => serde_json::json!({"status": "connecting"}),
                DownloadProgress::Downloading {
                    bytes_done,
                    bytes_total,
                } => serde_json::json!({
                    "status": "downloading",
                    "bytesDone": bytes_done,
                    "bytesTotal": bytes_total,
                }),
                DownloadProgress::Extracting => serde_json::json!({"status": "extracting"}),
                DownloadProgress::Done => serde_json::json!({"status": "done"}),
            };
            if let Ok(mut s) = status.lock() {
                s.progress = Some(payload.clone());
            }
            let _ = emit_handle.emit("download-progress", payload);
        })
            .map_err(|e| {
                format!(
                    "Could not download the built-in data files. Oxeylyzer needs them for \
                 layouts and language data — check your network connection and try \
                 again. {e}"
                )
            })?;
    }

    // Idempotent; ensure_data already calls it on first run, but calling it here means
    // a missing config is always recovered.
    let config_file = dirs.config_file();
    dirs.ensure_config()
        .map_err(|e| format!("Could not write {}. {e}", config_file.display()))?;

    let config = Config::with_loaded_weights(&config_file)
        .map_err(|e| format!("Could not parse {}. Fix or delete it to start over. {e}", config_file.display()))?;

    let corpus_path = dirs.data_dir().join(&config.corpus);
    let data = Data::load(&corpus_path).map_err(|e| {
        format!(
            "Could not read the corpus file {}. It is set by the `corpus` option in {} — \
             change it there, or pick a different language in the Language view. {e}",
            corpus_path.display(),
            config_file.display()
        )
    })?;

    let engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let layouts = load_all_layouts(&config, dirs.data_dir());

    Ok(Initialized {
        state: AppState {
            engine: Mutex::new(engine),
            layouts: Mutex::new(layouts),
            generated: Mutex::new(Vec::new()),
            dirs,
            config: Mutex::new(config),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            generating: Arc::new(AtomicBool::new(false)),
        },
    })
}

/// Starts the desktop application.
///
/// Returns the framework error rather than panicking on it, so the caller can set a
/// meaningful exit code. Startup failures that reach the user are reported in-window
/// by the setup hook and do not come through here.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Managed before any fallible work so the frontend can always query status,
            // even when startup fails.
            let status = Arc::new(Mutex::new(StartupStatus::default()));
            app.manage(Arc::clone(&status));

            let fail = |message: String| -> Result<(), Box<dyn std::error::Error>> {
                // The window already exists at this point — tauri builds configured
                // windows before invoking setup — so record the message for the UI and
                // keep the window open rather than tearing the app down.
                if let Ok(mut s) = status.lock() {
                    s.error = Some(message.clone());
                }
                let _ = app_handle.emit("startup-error", message);
                Ok(())
            };

            let dirs = match resolve_dirs() {
                Ok(dirs) => dirs,
                Err(message) => return fail(message),
            };

            match initialize(dirs, app_handle.clone(), Arc::clone(&status)) {
                Ok(initialized) => {
                    app.manage(initialized.state);
                }
                Err(message) => return fail(message),
            }

            let state = app.state::<AppState>();
            let watch_config = state.dirs.config_file();
            let watch_layouts = state.dirs.layouts_dir();

            // File watcher: auto-reload when config.toml or layout files change.
            {
                use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
                    let mut watcher = match recommended_watcher(tx) {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("File watcher init failed: {e}");
                            return;
                        }
                    };
                    let _ = watcher.watch(&watch_config, RecursiveMode::NonRecursive);
                    let _ = watcher.watch(&watch_layouts, RecursiveMode::Recursive);
                    let mut last_reload = std::time::Instant::now()
                        .checked_sub(std::time::Duration::from_secs(5))
                        .unwrap_or_else(std::time::Instant::now);
                    for event in rx.into_iter().flatten() {
                        if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            continue;
                        }
                        if last_reload.elapsed() < std::time::Duration::from_secs(2) {
                            continue;
                        }
                        last_reload = std::time::Instant::now();
                        let state = app_handle.state::<AppState>();

                        // Only a config.toml change requires rebuilding the engine
                        // (which invalidates generated results). Layout file changes
                        // — including the app's own saves — just refresh the layout
                        // map so in-progress work (e.g. generation results) survives.
                        let config_changed = event.paths.iter().any(|p| p.ends_with("config.toml"));
                        if config_changed {
                            if let Err(e) = reload_state(&state) {
                                eprintln!("Auto-reload failed: {e}");
                            } else {
                                let _ = app_handle.emit("config-reloaded", ());
                            }
                        } else {
                            let config = state.config.lock().unwrap().clone();
                            *state.layouts.lock().unwrap() =
                                load_all_layouts(&config, state.dirs.data_dir());
                            let _ = app_handle.emit("layouts-reloaded", ());
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_layouts,
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
            startup_error,
            download_progress,
            list_weight_presets,
            save_weight_preset,
            load_weight_preset,
        ])
        .run(tauri::generate_context!())
}
