//! Session-adjacent spill directory (atao5).

use std::path::{Path, PathBuf};

/// Install process-wide spill root for tool hard-truncation via app/core seam.
pub fn install_process_spill_dir(dir: Option<PathBuf>) {
    crate::app::core::tool_spill::install(dir);
}

/// Current process spill root (tests).
pub fn process_spill_dir() -> Option<PathBuf> {
    crate::app::core::tool_spill::current()
}

/// Unique spill directory next to session store: `{sessions_dir}/{session_id}.spill/`.
pub fn session_spill_dir(sessions_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir.join(format!("{session_id}.spill"))
}

/// Ensure the session spill directory exists; return its path.
pub fn ensure_session_spill_dir(sessions_dir: &Path, session_id: &str) -> Result<PathBuf, String> {
    let dir = session_spill_dir(sessions_dir, session_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create spill dir: {e}"))?;
    Ok(dir)
}

/// Write full text into the spill directory; return absolute path.
pub fn write_spill_file(spill_dir: &Path, stem: &str, full_text: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(spill_dir).map_err(|e| format!("create spill dir: {e}"))?;
    let path = spill_dir.join(format!("{stem}-{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&path, full_text).map_err(|e| format!("write spill: {e}"))?;
    Ok(path)
}

/// Cap body to `max_lines` logical lines; when overflowing, spill full text and
/// append a `[Full output: …]` footer (atao4 / atao5).
pub fn cap_with_spill(
    full_text: &str,
    max_lines: usize,
    spill_dir: &Path,
    stem: &str,
) -> Result<(String, Option<PathBuf>), String> {
    let lines: Vec<&str> = full_text.lines().collect();
    if lines.len() <= max_lines {
        return Ok((full_text.to_string(), None));
    }
    let path = write_spill_file(spill_dir, stem, full_text)?;
    let shown: Vec<&str> = lines.into_iter().take(max_lines).collect();
    let body = shown.join("\n");
    let path_str = path.to_string_lossy();
    let display = format!(
        "{body}\n[Full output: {path_str}. Truncated: {max_lines} lines shown (append-only cap)]"
    );
    Ok((display, Some(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_spill_is_beside_sessions_dir() {
        let dir = session_spill_dir(Path::new("/tmp/sessions"), "abc");
        assert_eq!(dir, PathBuf::from("/tmp/sessions/abc.spill"));
    }

    #[test]
    fn cap_with_spill_writes_and_annotates() {
        let tmp = tempfile::tempdir().unwrap();
        let spill = tmp.path().join("sid.spill");
        let full = "a\nb\nc\nd\ne\n";
        let (display, path) = cap_with_spill(full, 3, &spill, "assistant").unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.starts_with(&spill));
        assert!(display.contains("[Full output:"));
        assert!(display.lines().count() >= 4); // 3 body + footer
        assert_eq!(std::fs::read_to_string(&path).unwrap(), full);
    }

    #[test]
    fn process_spill_dir_roundtrip() {
        install_process_spill_dir(Some(PathBuf::from("/tmp/xy-spill-test")));
        assert_eq!(
            process_spill_dir(),
            Some(PathBuf::from("/tmp/xy-spill-test"))
        );
        install_process_spill_dir(None);
        assert_eq!(process_spill_dir(), None);
    }
}
