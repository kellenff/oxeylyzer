use std::io::Read;
use std::path::Path;

use crate::ResourceError;

const DATA_URL: &str = "https://github.com/o-x-e-y/oxeylyzer/archive/refs/heads/data-files.zip";

// GitHub archive zip puts all files under a root folder named "{repo}-{branch}/"
const ZIP_INNER_PREFIX: &str = "oxeylyzer-data-files/";

/// Written to the data root once extraction succeeds, so a failed or partial
/// extraction is never mistaken for a complete install.
const DATA_VERSION: &str = "1.0.0";

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

    let resp = ureq::get(DATA_URL)
        .call()
        .map_err(|e| ResourceError::Download(e.to_string()))?;

    let content_length = resp
        .header("content-length")
        .and_then(|v| v.parse::<u64>().ok());

    let mut body = Vec::new();
    let mut bytes_done: u64 = 0;
    let mut reader = resp.into_reader();
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

        // `enclosed_name` rejects NUL bytes, absolute paths, drive prefixes, and any
        // `..` that would climb out of the archive root. The prefix is stripped only
        // afterwards: the raw name's own directory component supplies the depth that a
        // following `../..` would consume, so validating the whole name first would let
        // `oxeylyzer-data-files/../../etc/passwd` through.
        let enclosed = file.enclosed_name().ok_or_else(|| {
            ResourceError::Extraction(format!(
                "archive entry escapes destination: {}",
                file.name()
            ))
        })?;

        let raw_name = enclosed.to_string_lossy();
        let stripped = raw_name.strip_prefix(ZIP_INNER_PREFIX).unwrap_or(&raw_name);
        if stripped.is_empty() {
            continue;
        }

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

    std::fs::write(dest.join(".data-version"), DATA_VERSION)?;

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
    fn traversal_escaping_root_is_rejected() {
        let (root, dest, ok) = extract(&[("oxeylyzer-data-files/../../../escaped.txt", "pwned")]);
        assert!(!ok, "traversal entry must not extract");
        assert!(
            !root.parent().unwrap().join("escaped.txt").exists(),
            "file was written outside the destination"
        );
        assert!(!dest.join("escaped.txt").exists());
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
}
