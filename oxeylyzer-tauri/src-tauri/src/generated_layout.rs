use std::{io::Write, path::Path};

pub(super) fn write_new(path: &Path, json: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "Layout {} already exists; choose another name",
                    path.display()
                )
            } else {
                format!("Write failed: {e}")
            }
        })?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn directory() -> std::path::PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "oxeylyzer-save-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn existing_file_is_preserved() {
        let dir = directory();
        let path = dir.join("gen-1.dof");
        std::fs::write(&path, b"original\0bytes").unwrap();
        let error = write_new(&path, "replacement").unwrap_err();
        assert!(error.contains("already exists"));
        assert_eq!(std::fs::read(&path).unwrap(), b"original\0bytes");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn new_file_is_written_and_repeat_refused() {
        let dir = directory();
        let path = dir.join("nested/gen-1.dof");
        write_new(&path, "serialized layout").unwrap();
        assert!(write_new(&path, "replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"serialized layout");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_parent_is_reported() {
        let dir = directory();
        let parent = dir.join("file");
        std::fs::write(&parent, b"keep").unwrap();
        assert!(write_new(&parent.join("layout.dof"), "layout").is_err());
        assert_eq!(std::fs::read(parent).unwrap(), b"keep");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concurrent_saves_have_one_winner() {
        let dir = directory();
        let path = dir.join("layout.dof");
        let results = std::thread::scope(|s| {
            let a = s.spawn(|| write_new(&path, "first"));
            let b = s.spawn(|| write_new(&path, "second"));
            (a.join().unwrap(), b.join().unwrap())
        });
        assert_ne!(results.0.is_ok(), results.1.is_ok());
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes == b"first" || bytes == b"second");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
