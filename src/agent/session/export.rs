//! SessionExporter — HTML/JSONL export and JSONL import (spec c255 / as32).
//!
//! Extracted from `AgentSession` (per the revised `architecture/ar02` upper
//! bound) into pure transformation helpers over a [`SessionManager`].
//! `AgentSession` delegates to these, preserving the public API (as31).

use crate::infra::session::SessionEntry;
use crate::infra::session::manager::SessionManager;

/// Export the entries of `session_id` to an HTML file. Returns the written path.
pub async fn export_to_html(
    manager: &SessionManager,
    session_id: &str,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let entries = manager.load(session_id).await?;
    let html = crate::infra::session::export::render_html(session_id, &entries);
    crate::infra::session::export::write_to(path, &html)?;
    Ok(path.to_path_buf())
}

/// Export the entries of `session_id` as JSONL. Returns the written path.
pub async fn export_to_jsonl(
    manager: &SessionManager,
    session_id: &str,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let entries = manager.load(session_id).await?;
    let jsonl = crate::infra::session::export::render_jsonl(&entries)?;
    crate::infra::session::export::write_to(path, &jsonl)?;
    Ok(path.to_path_buf())
}

/// Import a JSONL file into a brand-new session. Returns the new session id.
///
/// The new session id is derived from the source header (re-used) to keep
/// identities stable across export/import; the file lands in `manager`'s
/// sessions dir without overwriting an existing session.
pub async fn import_from_jsonl(
    manager: &SessionManager,
    path: &std::path::Path,
) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let entries = crate::infra::session::export::parse_jsonl(&bytes)?;
    let new_id = match entries.first() {
        Some(SessionEntry::Header(h)) => h.id.clone(),
        _ => return Err("import: missing header".into()),
    };
    if manager.exists(&new_id) {
        return Err(format!("session already exists: {new_id}"));
    }
    for entry in &entries {
        manager.append(&new_id, entry).await?;
    }
    Ok(new_id)
}

/// Share guidance message (no network upload).
pub fn share_as_gist(path: &std::path::Path) -> String {
    crate::infra::session::export::share_guidance_message(path)
}
