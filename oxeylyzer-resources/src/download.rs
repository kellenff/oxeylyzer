use std::io::Read;
use std::path::Path;

use crate::ResourceError;

const DATA_URL: &str = "https://github.com/o-x-e-y/oxeylyzer/archive/refs/heads/data-files.zip";

// GitHub archive zip puts all files under a root folder named "{repo}-{branch}/"
const ZIP_INNER_PREFIX: &str = "oxeylyzer-data-files/";

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

    extract_archive(&body, dest)?;

    std::fs::write(dest.join(".data-version"), "1.0.0")?;

    progress(DownloadProgress::Done);
    Ok(())
}

fn extract_archive(body: &[u8], dest: &Path) -> Result<(), ResourceError> {
    let cursor = std::io::Cursor::new(body);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ResourceError::Extraction(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ResourceError::Extraction(e.to_string()))?;

        let raw_name = file.name().to_string();
        let unsafe_path = |name: &str| {
            name.contains(['\\', ':'])
                || Path::new(name)
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
        };
        if unsafe_path(&raw_name) {
            return Err(ResourceError::Extraction(format!(
                "Unsafe archive path: {raw_name}"
            )));
        }
        let stripped = match raw_name.strip_prefix(ZIP_INNER_PREFIX) {
            Some(s) => s,
            None => &raw_name,
        };
        if stripped.is_empty() {
            continue;
        }

        if unsafe_path(stripped) {
            return Err(ResourceError::Extraction(format!(
                "Unsafe archive path: {raw_name}"
            )));
        }
        // Reserve root dotfiles so filesystem aliases cannot forge the completion marker.
        if stripped.starts_with('.') {
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn failed_archive_cannot_create_or_replace_completion_marker() {
        let root = std::env::temp_dir().join(format!(
            "oxeylyzer-archive-marker-test-{}",
            std::process::id()
        ));
        let dest = root.join("data");
        let marker = dest.join(".data-version");
        std::fs::create_dir_all(&dest).unwrap();

        for name in [
            ".data-version",
            "oxeylyzer-data-files/.data-version",
            "oxeylyzer-data-files/.data-version/nested.txt",
            "oxeylyzer-data-files/.DATA-VERSION",
            "oxeylyzer-data-files/.data-version.",
            "oxeylyzer-data-files/.data-version ",
            ".data-verſion",
            "oxeylyzer-data-files/.data-verſion",
            "oxeylyzer-data-files/.data-verſion/nested.txt",
        ] {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            for entry in [name, "oxeylyzer-data-files/../escaped.txt"] {
                zip.start_file(entry, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"incomplete").unwrap();
            }
            let bytes = zip.finish().unwrap().into_inner();

            assert!(extract_archive(&bytes, &dest).is_err());
            assert!(!marker.exists(), "archive created marker from {name}");
            assert!(
                std::fs::read_dir(&dest).unwrap().next().is_none(),
                "archive extracted reserved path {name}"
            );

            std::fs::write(&marker, "existing").unwrap();
            assert!(extract_archive(&bytes, &dest).is_err());
            assert_eq!(std::fs::read_to_string(&marker).unwrap(), "existing");
            std::fs::remove_file(&marker).unwrap();
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_paths_stay_inside_destination() {
        let root =
            std::env::temp_dir().join(format!("oxeylyzer-archive-test-{}", std::process::id()));
        let dest = root.join("data");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(root.join("escaped.txt"), "sentinel").unwrap();
        for name in [
            "oxeylyzer-data-files/../escaped.txt",
            "../escaped.txt",
            "/absolute",
            "C:/escape",
            "a\\..\\escape",
        ] {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"bad").unwrap();
            let bytes = zip.finish().unwrap().into_inner();
            assert!(extract_archive(&bytes, &dest).is_err(), "accepted {name}");
            assert_eq!(
                std::fs::read_to_string(root.join("escaped.txt")).unwrap(),
                "sentinel"
            );
        }
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.add_directory(
            "oxeylyzer-data-files/",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.start_file(
            "oxeylyzer-data-files/.data-version",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"untrusted").unwrap();
        zip.start_file(
            "oxeylyzer-data-files/nested/file.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"ok").unwrap();
        zip.start_file(
            "oxeylyzer-data-files/nested/.data-version",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"nested resource").unwrap();
        extract_archive(&zip.finish().unwrap().into_inner(), &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("nested/file.txt")).unwrap(),
            "ok"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("nested/.data-version")).unwrap(),
            "nested resource"
        );
        assert!(!dest.join(".data-version").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
