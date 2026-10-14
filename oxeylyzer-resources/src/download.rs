use std::io::Read;
use std::path::{Component, Path};

use crate::ResourceError;

const DATA_URL: &str = "https://github.com/o-x-e-y/oxeylyzer/archive/refs/heads/data-files.zip";

// GitHub archive zip puts all files under a root folder named "{repo}-{branch}/"
const ZIP_INNER_PREFIX: &str = "oxeylyzer-data-files/";

/// Name of the sentinel file in the data root recording the installed data version.
pub const DATA_VERSION_FILE: &str = ".data-version";

/// Data files version this build expects.
///
/// Bumped by hand when the published data release changes. Deliberately not
/// derived from the crate version: a bugfix release must not re-download every
/// user's data files.
pub const DATA_VERSION: &str = "1.0.0";

pub enum DownloadProgress {
    Connecting,
    Downloading {
        bytes_done: u64,
        bytes_total: Option<u64>,
    },
    Extracting,
    Done,
}

pub fn download_and_extract<F>(dest: &Path, progress: F) -> Result<(), ResourceError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    progress(DownloadProgress::Connecting);

    let mut resp = ureq::get(DATA_URL)
        .call()
        .map_err(|e| ResourceError::Download(e.to_string()))?;

    let content_length = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    let mut body = Vec::new();
    let mut bytes_done: u64 = 0;
    let mut reader = resp.body_mut().as_reader();
    let mut buf = [0u8; 16384];

    loop {
        let n = reader.read(&mut buf).map_err(ResourceError::Io)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
        bytes_done += n as u64;
        progress(DownloadProgress::Downloading {
            bytes_done,
            bytes_total: content_length,
        });
    }

    progress(DownloadProgress::Extracting);

    extract_bytes(&body, dest)?;

    progress(DownloadProgress::Done);
    Ok(())
}

/// Whether a relative entry path stays at or below its own root.
///
/// Applied to the raw archive name after [`ZIP_INNER_PREFIX`] is stripped: the
/// prefix banks one level of depth, so a single `../` spent on it must not read as
/// in-bounds. Absolute paths, drive prefixes, NUL bytes, and any `..` that would
/// climb out of the root are refused.
fn is_contained(rel: &str) -> bool {
    let mut depth = 0usize;
    for component in Path::new(rel).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::ParentDir => match depth.checked_sub(1) {
                Some(next) => depth = next,
                None => return false,
            },
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
        }
    }
    true
}

/// Unpacks a zip archive into `dest`, refusing any entry that would land outside it.
///
/// Kept separate from the download so the containment rules can be exercised
/// without reaching the network.
fn extract_bytes(body: &[u8], dest: &Path) -> Result<(), ResourceError> {
    let cursor = std::io::Cursor::new(body);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ResourceError::Extraction(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ResourceError::Extraction(e.to_string()))?;

        // Containment is decided here, on the raw entry name — never through
        // `ZipFile::enclosed_name`. That method's semantics are a moving target: zip 2
        // rejected a `..` that climbed out of the archive root, while zip 9 normalizes
        // it away instead (`oxeylyzer-data-files/../escaped.txt` becomes plain
        // `escaped.txt`, and a leading `/` or drive prefix silently loses its root).
        // Both count as "safe" by zip's own rules — files stay inside `dest` — but
        // normalization would let this loop write to paths the archive never claimed.
        // Checking the raw name keeps the policy ours instead of inheriting whatever
        // upstream calls safe next version.
        let escapes = |name: &str| {
            ResourceError::Extraction(format!("archive entry escapes destination: {name}"))
        };
        let raw_name = file
            .name()
            .map_err(|e| ResourceError::Extraction(e.to_string()))?
            .into_owned();

        // `Path::components()` drops NUL bytes instead of refusing them, which
        // `enclosed_name()` used to reject; check the raw name before anything else.
        if raw_name.contains('\0') {
            return Err(escapes(&raw_name));
        }

        // Strip the prefix as a string operation, before any path normalization:
        // `oxeylyzer-data-files//tmp/x` collapses to `tmp/x` when read as a Path, but
        // leaves an absolute `/tmp/x` remainder here, which `is_contained` then refuses.
        let stripped = raw_name
            .strip_prefix(ZIP_INNER_PREFIX)
            .unwrap_or(raw_name.as_str());
        if stripped.is_empty() {
            continue;
        }
        if !is_contained(stripped) {
            return Err(escapes(&raw_name));
        }

        // shortcut: entries written before a mid-archive refusal stay on disk;
        // the `.data-version` sentinel (written only after full success) is what
        // marks an install complete. Upgrade to temp-dir staging if that ever ships.
        let out_path = dest.join(stripped);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }

    std::fs::write(dest.join(DATA_VERSION_FILE), DATA_VERSION)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an in-memory zip from `(name, contents)` pairs.
    ///
    /// A name ending in `/` becomes a directory entry, matching how GitHub archive
    /// zips record their root folder.
    fn archive(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (name, contents) in entries {
            if name.ends_with('/') {
                writer.add_directory(*name, options).unwrap();
            } else {
                writer.start_file(*name, options).unwrap();
                std::io::Write::write_all(&mut writer, contents.as_bytes()).unwrap();
            }
        }
        writer.finish().unwrap().into_inner()
    }

    /// Extracts `entries` into a fresh temp directory and returns `(root, dest, result)`.
    fn extract(entries: &[(&str, &str)]) -> (std::path::PathBuf, std::path::PathBuf, bool) {
        let root = std::env::temp_dir().join(format!(
            "oxeylyzer-extract-{}-{}",
            std::process::id(),
            next_id()
        ));
        let dest = root.join("data");
        std::fs::create_dir_all(&dest).unwrap();
        let bytes = archive(entries);
        let result = extract_bytes(&bytes, &dest);
        (root, dest, result.is_ok())
    }

    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn next_id() -> usize {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Relative paths of every regular file under `dir`, sorted for comparison.
    fn files_under(dir: &Path) -> Vec<String> {
        let mut found = Vec::new();
        walk(dir, dir, &mut found);
        found.sort();
        found
    }

    /// Relative paths of files under `root` that landed outside `dest` (`root/data`).
    fn files_outside(root: &Path) -> Vec<String> {
        files_under(root)
            .into_iter()
            .filter(|path| !path.starts_with("data/"))
            .collect()
    }

    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    #[test]
    fn files_outside_reports_everything_outside_the_destination() {
        let (root, _, _) = extract(&[("oxeylyzer-data-files/a.txt", "inside")]);
        // Extraction succeeded and only `data/a.txt` should exist.
        assert_eq!(files_outside(&root), Vec::<String>::new());

        std::fs::write(root.join("planted.txt"), "outside").unwrap();
        std::fs::create_dir_all(root.join("someotherapp")).unwrap();
        std::fs::write(root.join("someotherapp/payload.txt"), "outside").unwrap();

        let mut expected = vec![
            "planted.txt".to_string(),
            "someotherapp/payload.txt".to_string(),
        ];
        expected.sort();
        assert_eq!(files_outside(&root), expected);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn traversal_escaping_root_is_rejected() {
        let (root, _, ok) = extract(&[("oxeylyzer-data-files/../../../escaped.txt", "pwned")]);
        assert!(!ok, "traversal entry must not extract");
        assert_eq!(
            files_outside(&root),
            Vec::<String>::new(),
            "file was written outside the destination"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// zip-9's `enclosed_name()` *normalizes* `oxeylyzer-data-files/../escaped.txt`
    /// into `escaped.txt` instead of rejecting it. The containment decision must come
    /// from `is_contained` on the RAW name, so the archive never gets to rewrite what
    /// it claimed. FR-013, the zip-9 upgrade guard.
    #[test]
    fn zip9_normalization_rejection() {
        let (root, dest, ok) = extract(&[("oxeylyzer-data-files/../escaped.txt", "pwned")]);
        assert!(
            !ok,
            "raw-name containment must reject the normalizable entry"
        );
        assert!(
            !dest.join("escaped.txt").exists(),
            "refusal must not degrade into writing dest/escaped.txt under the claimed name"
        );
        assert_eq!(files_outside(&root), Vec::<String>::new());
        std::fs::remove_dir_all(&root).ok();
    }

    /// Single `..` after the prefix is the shape the shipped order let through: the
    /// prefix banks one level of depth, which one `..` then spends. This is the
    /// regression test for the ordering bug — it must land at `dest/../escaped.txt`.
    #[test]
    fn single_parent_after_prefix_is_rejected() {
        let (root, _, ok) = extract(&[("oxeylyzer-data-files/../escaped.txt", "pwned")]);
        assert!(!ok, "single `..` after the prefix must not extract");
        assert_eq!(
            files_outside(&root),
            Vec::<String>::new(),
            "file was written one level above the destination"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn padded_single_parent_after_prefix_is_rejected() {
        let (root, _, ok) = extract(&[("oxeylyzer-data-files/a/b/../../../escaped.txt", "pwned")]);
        assert!(
            !ok,
            "padding with in-bounds segments must not launder a `..`"
        );
        assert_eq!(files_outside(&root), Vec::<String>::new());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn sideways_write_above_the_destination_is_rejected() {
        let (root, _, ok) = extract(&[(
            "oxeylyzer-data-files/../someotherapp/evil/payload.txt",
            "pwned",
        )]);
        assert!(!ok, "sibling of the data root must not be writable");
        assert_eq!(files_outside(&root), Vec::<String>::new());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn absolute_path_entry_is_rejected() {
        // `dest.join("/abs")` discards `dest` entirely, so an absolute entry writes to
        // that literal path. Point it somewhere writable so a missing guard shows up as
        // a created file rather than a permission error.
        let root = std::env::temp_dir().join(format!(
            "oxeylyzer-abs-{}-{}",
            std::process::id(),
            next_id()
        ));
        let dest = root.join("data");
        std::fs::create_dir_all(&dest).unwrap();
        let outside = root.join("absolute-escape.txt");

        let bytes = archive(&[(outside.to_str().unwrap(), "pwned")]);
        let result = extract_bytes(&bytes, &dest);

        assert!(result.is_err(), "absolute entry must not extract");
        assert!(
            !outside.exists(),
            "absolute entry was written to its literal path"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A second `/` after the prefix survives `enclosed_name()` (its component walk
    /// collapses repeated separators) but leaves a *rooted* remainder once the prefix
    /// is stripped, so `dest.join` discards `dest` and writes to the literal path.
    /// This is the absolute-path variant of the same ordering bug.
    #[test]
    fn doubled_separator_after_prefix_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "oxeylyzer-double-{}-{}",
            std::process::id(),
            next_id()
        ));
        let dest = root.join("data");
        std::fs::create_dir_all(&dest).unwrap();
        let outside = root.join("absolute-escape.txt");

        let entry = format!("{ZIP_INNER_PREFIX}{}", outside.to_str().unwrap());
        let result = extract_bytes(&archive(&[(entry.as_str(), "pwned")]), &dest);

        assert!(result.is_err(), "rooted remainder must not extract");
        assert!(
            !outside.exists(),
            "rooted remainder was written to its literal path"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn parent_segment_within_bounds_is_allowed() {
        let (root, dest, ok) = extract(&[("oxeylyzer-data-files/a/../b.txt", "ok")]);
        assert!(ok, "traversal that stays inside must extract");
        assert!(dest.join("b.txt").exists(), "expected dest/b.txt");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn prefixed_entry_has_prefix_stripped() {
        let (root, dest, ok) = extract(&[("oxeylyzer-data-files/static/text/en.txt", "hello")]);
        assert!(ok);
        assert_eq!(
            std::fs::read_to_string(dest.join("static/text/en.txt")).unwrap(),
            "hello"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unprefixed_entry_extracts_at_root() {
        let (root, dest, ok) = extract(&[("static/text/en.txt", "hello")]);
        assert!(ok);
        assert_eq!(
            std::fs::read_to_string(dest.join("static/text/en.txt")).unwrap(),
            "hello"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn archive_root_entry_is_skipped() {
        let (root, dest, ok) = extract(&[
            ("oxeylyzer-data-files/", ""),
            ("oxeylyzer-data-files/en.txt", "hello"),
        ]);
        assert!(ok, "root directory entry must not be an error");
        assert!(dest.join("en.txt").exists());
        assert!(
            !dest.join("oxeylyzer-data-files").exists(),
            "prefix must be stripped, not recreated as a directory"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn mixed_valid_and_hostile_fails_without_partial_files() {
        let (root, dest, ok) = extract(&[
            ("oxeylyzer-data-files/good.txt", "fine"),
            ("oxeylyzer-data-files/../../escape.txt", "pwned"),
        ]);
        assert!(!ok, "one hostile entry must abort the whole extraction");
        assert!(
            !dest.join(".data-version").exists(),
            "sentinel must not be written when extraction aborts, or the next \
             startup mistakes a partial install for a complete one"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The hostile entry FIRST: every later valid entry is skipped, and the sentinel
    /// invariant holds in this order too — `.data-version` absent whenever any entry
    /// was refused (FR-014, edge case "hostile entry appears before valid one").
    #[test]
    fn hostile_entry_first_aborts_without_sentinel() {
        let (root, dest, ok) = extract(&[
            ("oxeylyzer-data-files/../../escape.txt", "pwned"),
            ("oxeylyzer-data-files/good.txt", "fine"),
        ]);
        assert!(!ok, "leading hostile entry must abort extraction");
        assert!(
            !dest.join(".data-version").exists(),
            "sentinel must be absent when an entry was refused"
        );
        assert!(
            !dest.join("good.txt").exists(),
            "entries after the refusal must not be written"
        );
        assert_eq!(files_outside(&root), Vec::<String>::new());
        std::fs::remove_dir_all(&root).ok();
    }
}
