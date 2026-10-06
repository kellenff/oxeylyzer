use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{
    download::{DownloadProgress, DATA_VERSION, DATA_VERSION_FILE},
    seed, ResourceError,
};

/// Manages platform-appropriate paths for oxeylyzer configuration and data.
///
/// Follows XDG conventions on Linux — configuration lives in `$XDG_CONFIG_HOME`
/// and downloaded/generated data lives in `$XDG_DATA_HOME`:
///
/// | Location            | Linux                              | Windows/macOS          |
/// |---------------------|------------------------------------|------------------------|
/// | `config_root`       | `$XDG_CONFIG_HOME/oxeylyzer`       | `%APPDATA%\oxeylyzer`  |
/// | `data_root`         | `$XDG_DATA_HOME/oxeylyzer`         | same as config_root    |
///
/// **Config root** holds: `config.toml`, `session.json`, `history.txt`, `weight-presets/`
/// **Data root** holds:   `static/layouts/`, `static/language_data/`, `static/corpus_configs/`
///
/// For development set `OXEYLYZER_DATA_DIR=.` (workspace root); both roots collapse to
/// the same directory so the existing `./static/` tree is used directly.
#[derive(Clone)]
pub struct OxeylyzerDirs {
    config_root: PathBuf,
    data_root: PathBuf,
}

/// Whether an installation's data files match the version this build expects.
///
/// Distinguishes "never downloaded" from "downloaded, but behind" so callers can
/// explain the difference rather than only reporting a yes/no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataCurrency {
    /// No version recorded — either a fresh installation or an unreadable sentinel.
    Absent,
    /// A version is recorded and it is older than the current one.
    Stale { installed: String },
    /// A version is recorded and it matches or exceeds the current one.
    ///
    /// A recorded version *newer* than current lands here too: the user downgraded
    /// the app, and their data is still usable.
    Current { installed: String },
}

impl DataCurrency {
    /// Whether data must be downloaded.
    pub fn needs_download(&self) -> bool {
        !matches!(self, DataCurrency::Current { .. })
    }
}

/// Orders two dot-separated versions, e.g. `"1.0.1" > "1.0.0"`.
///
/// A longer version is newer when the components it shares all match, so `"1.0"`
/// is older than `"1.0.1"`. Returns `None` when either side is not a
/// dot-separated list of numbers — callers treat that as "unknown", which for the
/// installed version means download rather than assume currency.
fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<Vec<u64>> {
        if s.is_empty() {
            return None;
        }
        s.split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<u64>, _>>()
            .ok()
    };

    let a = parse(a)?;
    let b = parse(b)?;

    for (x, y) in a.iter().zip(&b) {
        match x.cmp(y) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    Some(a.len().cmp(&b.len()))
}

/// Where this platform says the application's files live, without creating anything.
///
/// Separated from [`OxeylyzerDirs::resolve`] so path containment can be verified
/// against the real platform directories without writing to the user's config or
/// data locations.
fn platform_roots() -> Option<(PathBuf, PathBuf)> {
    let proj = ProjectDirs::from("", "", "oxeylyzer")?;
    Some((
        proj.config_dir().to_path_buf(),
        proj.data_dir().to_path_buf(),
    ))
}

/// Reads the recorded data version from the data root.
fn installed_version(data_root: &Path) -> Option<String> {
    std::fs::read_to_string(data_root.join(DATA_VERSION_FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl OxeylyzerDirs {
    /// Resolves platform directories and creates them if needed.
    pub fn resolve() -> Result<Self, ResourceError> {
        let (config_root, data_root) = platform_roots().ok_or(ResourceError::NoDirFound)?;

        std::fs::create_dir_all(&config_root)?;
        std::fs::create_dir_all(&data_root)?;

        Ok(Self {
            config_root,
            data_root,
        })
    }

    /// How current this installation's data files are.
    pub fn data_currency(&self) -> DataCurrency {
        let Some(installed) = installed_version(&self.data_root) else {
            return DataCurrency::Absent;
        };
        match compare_versions(&installed, DATA_VERSION) {
            Some(std::cmp::Ordering::Less) | None => DataCurrency::Stale { installed },
            Some(_) => DataCurrency::Current { installed },
        }
    }

    /// Whether data files must be downloaded — true when none are recorded or the
    /// recorded version is behind [`DATA_VERSION`].
    pub fn needs_data(&self) -> bool {
        self.data_currency().needs_download()
    }

    /// Creates an `OxeylyzerDirs` pointing at a single override root.
    ///
    /// Both config and data resolve under the same directory — intended for
    /// development (`OXEYLYZER_DATA_DIR=.`) and tests.
    pub fn with_override(root: PathBuf) -> Self {
        Self {
            config_root: root.clone(),
            data_root: root,
        }
    }

    // ── Data root (XDG_DATA_HOME) ────────────────────────────────────────────

    /// The data root directory — used to resolve relative paths from `config.toml`.
    pub fn data_dir(&self) -> &Path {
        &self.data_root
    }

    /// `data_root/static/layouts`
    pub fn layouts_dir(&self) -> PathBuf {
        self.data_root.join("static/layouts")
    }

    /// `data_root/static/language_data`
    pub fn language_data_dir(&self) -> PathBuf {
        self.data_root.join("static/language_data")
    }

    /// `data_root/static/corpus_configs`
    pub fn corpus_configs_dir(&self) -> PathBuf {
        self.data_root.join("static/corpus_configs")
    }

    /// `data_root/static/text`
    pub fn text_dir(&self) -> PathBuf {
        self.data_root.join("static/text")
    }

    // ── Config root (XDG_CONFIG_HOME) ────────────────────────────────────────

    /// `config_root/config.toml`
    pub fn config_file(&self) -> PathBuf {
        self.config_root.join("config.toml")
    }

    /// `config_root/session.json`
    pub fn session_file(&self) -> PathBuf {
        self.config_root.join("session.json")
    }

    /// `config_root/history.txt`
    pub fn history_file(&self) -> PathBuf {
        self.config_root.join("history.txt")
    }

    /// `config_root/weight-presets`
    pub fn weight_presets_dir(&self) -> PathBuf {
        self.config_root.join("weight-presets")
    }

    // ── Bootstrap ────────────────────────────────────────────────────────────

    /// Writes a default `config.toml` to the config root if one does not exist.
    ///
    /// Idempotent — safe to call on every startup before loading config.
    pub fn ensure_config(&self) -> Result<(), ResourceError> {
        seed::ensure_config(self)
    }

    /// Downloads and extracts data files if not already present, then ensures
    /// a default `config.toml` exists in the config root.
    ///
    /// Calls `progress` with download/extraction events. Blocks the current
    /// thread — wrap in `spawn_blocking` for async contexts.
    pub fn ensure_data<F>(&self, progress: F) -> Result<(), ResourceError>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        seed::ensure_data(self, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    fn temp_root(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        std::env::temp_dir().join(format!("oxeylyzer-{label}-{}-{n}", std::process::id()))
    }

    fn write_sentinel(root: &Path, contents: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(DATA_VERSION_FILE), contents).unwrap();
    }

    #[test]
    fn older_patch_is_less() {
        assert_eq!(compare_versions("1.0.0", "1.0.1"), Some(Ordering::Less));
    }

    #[test]
    fn older_minor_is_less() {
        assert_eq!(compare_versions("1.0.0", "1.1.0"), Some(Ordering::Less));
    }

    #[test]
    fn fewer_components_is_less() {
        assert_eq!(compare_versions("1.0", "1.0.1"), Some(Ordering::Less));
    }

    #[test]
    fn equal_versions() {
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Some(Ordering::Equal));
    }

    #[test]
    fn recorded_newer_than_current_is_greater() {
        // User downgraded the app; their data must not be rolled back.
        assert_eq!(compare_versions("2.0.0", "1.0.0"), Some(Ordering::Greater));
    }

    #[test]
    fn unparseable_versions_compare_to_none() {
        assert_eq!(compare_versions("abc", "1.0.0"), None);
        assert_eq!(compare_versions("1.0.0", "abc"), None);
        assert_eq!(compare_versions("", "1.0.0"), None);
        assert_eq!(compare_versions("1..0", "1.0.0"), None);
    }

    #[test]
    fn absent_sentinel_reports_absent_and_needs_download() {
        let root = temp_root("absent");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(dirs.data_currency(), DataCurrency::Absent);
        assert!(dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_sentinel_counts_as_absent() {
        let root = temp_root("empty");
        write_sentinel(&root, "");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(dirs.data_currency(), DataCurrency::Absent);
        assert!(dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unreadable_sentinel_version_is_treated_as_stale() {
        let root = temp_root("garbage");
        write_sentinel(&root, "not-a-version");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(
            dirs.data_currency(),
            DataCurrency::Stale {
                installed: "not-a-version".to_string()
            }
        );
        assert!(dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn current_version_needs_no_download() {
        let root = temp_root("current");
        write_sentinel(&root, DATA_VERSION);
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(
            dirs.data_currency(),
            DataCurrency::Current {
                installed: DATA_VERSION.to_string()
            }
        );
        assert!(!dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn stale_version_needs_download() {
        let root = temp_root("stale");
        write_sentinel(&root, "0.0.1");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert!(matches!(dirs.data_currency(), DataCurrency::Stale { .. }));
        assert!(dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn newer_recorded_version_needs_no_download() {
        // Downgrade case: data is ahead of what this build expects, still usable.
        let root = temp_root("newer");
        write_sentinel(&root, "99.0.0");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(
            dirs.data_currency(),
            DataCurrency::Current {
                installed: "99.0.0".to_string()
            }
        );
        assert!(!dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn needs_download_matches_currency_variant() {
        assert!(DataCurrency::Absent.needs_download());
        assert!(DataCurrency::Stale {
            installed: "0.0.1".into()
        }
            .needs_download());
        assert!(!DataCurrency::Current {
            installed: "1.0.0".into()
        }
            .needs_download());
    }

    #[test]
    fn platform_derived_paths_all_sit_under_platform_roots() {
        let Some((config_root, data_root)) = platform_roots() else {
            // No conventional location on this platform; nothing to assert.
            return;
        };
        let dirs = OxeylyzerDirs {
            config_root,
            data_root,
        };

        for path in [
            dirs.data_dir(),
            dirs.layouts_dir().as_path(),
            dirs.language_data_dir().as_path(),
            dirs.corpus_configs_dir().as_path(),
            dirs.text_dir().as_path(),
            dirs.config_file().as_path(),
            dirs.session_file().as_path(),
            dirs.history_file().as_path(),
            dirs.weight_presets_dir().as_path(),
        ] {
            let in_config = path.starts_with(&dirs.config_root);
            let in_data = path.starts_with(&dirs.data_root);
            assert!(
                in_config || in_data,
                "{} escaped both platform roots",
                path.display()
            );
        }
    }

    #[test]
    fn platform_roots_are_reported_without_being_created() {
        let Some((config_root, data_root)) = platform_roots() else {
            return;
        };
        // Recording them must not have created anything; if the user already has an
        // install these pre-exist, which is fine -- the point is this call creates nothing.
        let existed_before = config_root.exists() && data_root.exists();
        let _ = platform_roots();
        assert!(
            existed_before || (!config_root.exists() && !data_root.exists()),
            "platform_roots() must not create directories"
        );
    }

    #[test]
    fn platform_paths_are_stable_across_calls() {
        let Some(first) = platform_roots() else {
            return;
        };
        let second = platform_roots().expect("platform roots reported inconsistently");
        assert_eq!(first, second, "platform roots must be stable across calls");
    }

    #[test]
    fn absent_sentinel_on_a_fresh_root_needs_data() {
        let root = temp_root("freshroot");
        std::fs::create_dir_all(&root).unwrap();
        let dirs = OxeylyzerDirs::with_override(root.clone());
        assert_eq!(dirs.data_currency(), DataCurrency::Absent);
        assert!(dirs.needs_data());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn every_path_resolves_under_the_data_root() {
        let root = temp_root("paths");
        let dirs = OxeylyzerDirs::with_override(root.clone());
        for path in [
            dirs.layouts_dir(),
            dirs.language_data_dir(),
            dirs.corpus_configs_dir(),
            dirs.text_dir(),
            dirs.config_file(),
            dirs.session_file(),
            dirs.history_file(),
            dirs.weight_presets_dir(),
        ] {
            assert!(
                path.starts_with(&root),
                "{} escaped the resolved root",
                path.display()
            );
        }
        std::fs::remove_dir_all(&root).ok();
    }
}
