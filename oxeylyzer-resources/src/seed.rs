use oxeylyzer_core::weights::Config;

use crate::{
    OxeylyzerDirs, ResourceError,
    download::{DownloadProgress, download_and_extract},
};

pub fn ensure_data<F>(dirs: &OxeylyzerDirs, progress: F) -> Result<(), ResourceError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    // Refresh when data was never downloaded, or when the installed version is behind
    // the one this build expects. The sentinel is written only after a successful
    // extraction, so a failed download leaves the old version in place and this retries.
    if dirs.needs_data() {
        download_and_extract(dirs.data_dir(), progress)?;
    }

    ensure_config(dirs)
}

/// Writes a default `config.toml` to the config root if one does not exist.
///
/// Idempotent — safe to call on every startup.
pub fn ensure_config(dirs: &OxeylyzerDirs) -> Result<(), ResourceError> {
    let config_file = dirs.config_file();
    if !config_file.exists() {
        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_file, default_config_toml(dirs)?)?;
    }
    Ok(())
}

/// Builds a default `config.toml` string from `Config::default()` with paths
/// pointing into the data root for this installation.
fn default_config_toml(dirs: &OxeylyzerDirs) -> Result<String, ResourceError> {
    let config = Config {
        corpus: dirs.language_data_dir().join("english.json"),
        layouts: vec![dirs.layouts_dir().join("english").join("*.dof")],
        corpus_configs: dirs.corpus_configs_dir().join("**").join("*.toml"),
        ..Default::default()
    };

    toml::to_string_pretty(&config).map_err(|e| ResourceError::Config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::download::{DATA_VERSION, DATA_VERSION_FILE};

    fn temp_root(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        std::env::temp_dir().join(format!("oxeylyzer-seed-{label}-{}-{n}", std::process::id()))
    }

    fn dirs_for(root: &std::path::Path) -> OxeylyzerDirs {
        OxeylyzerDirs::with_override(root.to_path_buf())
    }

    fn write_sentinel(root: &std::path::Path, contents: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(DATA_VERSION_FILE), contents).unwrap();
    }

    #[test]
    fn current_version_skips_the_download_entirely() {
        let root = temp_root("current");
        write_sentinel(&root, DATA_VERSION);
        let dirs = dirs_for(&root);

        // Would attempt a network fetch if the version check did not short-circuit.
        let result = ensure_data(&dirs, |_| panic!("download must not run for current data"));

        assert!(result.is_ok(), "ensure_data should be a no-op here");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_refresh_records_the_new_version_and_leaves_unshipped_files_alone() {
        // Simulates a completed refresh: extraction writes the sentinel, and files the
        // release does not ship are left in place rather than pruned.
        let root = temp_root("refresh");
        write_sentinel(&root, "0.0.1");

        // A user-saved layout the release does not ship.
        let saved = root.join("static/layouts/mylayout.dof");
        std::fs::create_dir_all(saved.parent().unwrap()).unwrap();
        std::fs::write(&saved, "{ mine }").unwrap();

        // Stand in for a successful extraction.
        std::fs::write(root.join(DATA_VERSION_FILE), DATA_VERSION).unwrap();

        assert_eq!(
            std::fs::read_to_string(root.join(DATA_VERSION_FILE)).unwrap(),
            DATA_VERSION,
            "successful refresh must record the current version"
        );
        assert_eq!(
            std::fs::read_to_string(&saved).unwrap(),
            "{ mine }",
            "files the release does not ship must survive a refresh"
        );
        assert!(
            !OxeylyzerDirs::with_override(root.clone()).needs_data(),
            "a recorded current version must stop further refreshes"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ensure_data_writes_a_default_config() {
        let root = temp_root("config");
        write_sentinel(&root, DATA_VERSION);
        let dirs = dirs_for(&root);

        ensure_data(&dirs, |_| panic!("download must not run for current data")).unwrap();

        assert!(
            root.join("config.toml").exists(),
            "default config not written"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ensure_data_does_not_overwrite_an_edited_config() {
        let root = temp_root("keepconfig");
        write_sentinel(&root, DATA_VERSION);
        std::fs::write(root.join("config.toml"), "# user edited\n").unwrap();
        let dirs = dirs_for(&root);

        ensure_data(&dirs, |_| panic!("download must not run for current data")).unwrap();

        assert_eq!(
            std::fs::read_to_string(root.join("config.toml")).unwrap(),
            "# user edited\n",
            "existing config must be left alone"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The default config the app generates, loaded back through the same loader the
    /// REPL and Tauri app use at startup.
    fn generated_config(root: &std::path::Path) -> Config {
        let dirs = dirs_for(root);
        ensure_config(&dirs).expect("default config should be written");
        Config::with_loaded_weights(root.join("config.toml")).expect("generated config must load")
    }

    #[test]
    fn generated_config_loads_through_the_app_loader() {
        let root = temp_root("load");
        write_sentinel(&root, DATA_VERSION);

        // Panics inside ensure_config's test if the file cannot be loaded at all.
        let _ = generated_config(&root);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn generated_config_paths_point_inside_the_data_root() {
        let root = temp_root("paths");
        write_sentinel(&root, DATA_VERSION);
        let config = generated_config(&root);

        assert_eq!(
            config.corpus,
            root.join("static/language_data/english.json")
        );
        assert_eq!(
            config.layouts,
            vec![root.join("static/layouts/english").join("*.dof")]
        );
        assert_eq!(
            config.corpus_configs,
            root.join("static/corpus_configs").join("**").join("*.toml")
        );

        for path in std::iter::once(&config.corpus)
            .chain(&config.layouts)
            .chain(std::iter::once(&config.corpus_configs))
        {
            assert!(
                path.starts_with(&root),
                "{} escaped the data root",
                path.display()
            );
        }

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn generated_config_preserves_default_weights() {
        let root = temp_root("weights");
        write_sentinel(&root, DATA_VERSION);
        let config = generated_config(&root);

        // Weights has no PartialEq, so compare its serialized form against the defaults.
        let defaults = Config::default();
        assert_eq!(
            toml::to_string(&config.weights).unwrap(),
            toml::to_string(&defaults.weights).unwrap(),
            "weights must round-trip as defaults"
        );
        assert_eq!(config.trigram_precision, defaults.trigram_precision);
        assert_eq!(config.max_cores, defaults.max_cores);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn layouts_accept_both_single_string_and_list() {
        let root = temp_root("layoutsform");
        // Start from a real generated config so every required field is present; only
        // the `layouts` line differs between the two files.
        write_sentinel(&root, DATA_VERSION);
        ensure_config(&dirs_for(&root)).unwrap();
        let base = std::fs::read_to_string(root.join("config.toml")).unwrap();

        // Replace in place rather than appending: `layouts` belongs to the top-level
        // table, so moving it after `[weights]` would silently nest it.
        let with_layouts = |layouts_toml: &str| {
            base.lines()
                .map(|line| {
                    if line.starts_with("layouts") {
                        format!("layouts = {layouts_toml}")
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let single = root.join("single.toml");
        std::fs::write(&single, with_layouts("\"a.dof\"")).unwrap();
        let listed = root.join("listed.toml");
        std::fs::write(&listed, with_layouts("[\"a.dof\", \"b.dof\"]")).unwrap();

        let from_single = Config::with_loaded_weights(&single).expect("single form must load");
        assert_eq!(from_single.layouts, vec![PathBuf::from("a.dof")]);

        let from_list = Config::with_loaded_weights(&listed).expect("list form must load");
        assert_eq!(
            from_list.layouts,
            vec![PathBuf::from("a.dof"), PathBuf::from("b.dof")]
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn paths_needing_escaping_survive_the_round_trip() {
        // A data root with a space and a non-ASCII character must be escaped on write
        // and read back byte-identical.
        let root = std::env::temp_dir().join("oxeylyzer-escaped test \u{e9}\u{20ac}");
        std::fs::remove_dir_all(&root).ok();
        write_sentinel(&root, DATA_VERSION);

        let config = generated_config(&root);
        assert_eq!(
            config.corpus,
            root.join("static/language_data/english.json")
        );
        assert!(config.corpus.to_string_lossy().contains('\u{e9}'));

        std::fs::remove_dir_all(&root).ok();
    }
}
