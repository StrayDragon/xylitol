//! Session JSONL parse and version enforcement.

use super::entries::{LEGACY_SESSION_VERSION, SESSION_VERSION, SessionEntry};
use crate::protocol::error::XySessionStoreError;

/// Parse session JSONL content: skip unparseable / non-SSOT lines with warn≤3 then `...`.
///
/// Requires a header with [`SESSION_VERSION`]; does not migrate older versions.
pub fn parse_session_jsonl(content: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let (entries, _) = parse_session_jsonl_lines(content);
    enforce_session_version(&entries)?;
    Ok(entries)
}

/// Line parse with skip/warn, without header-version enforcement (flush merge).
pub fn parse_session_jsonl_lines(content: &str) -> (Vec<SessionEntry>, usize) {
    let mut entries = Vec::new();
    let mut warn_count = 0usize;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => emit_session_load_warn(
                &mut warn_count,
                &format!("skip session line: parse entry: {e}"),
            ),
        }
    }
    (entries, warn_count)
}

fn emit_session_load_warn(warn_count: &mut usize, msg: &str) {
    *warn_count += 1;
    if *warn_count <= 3 {
        log::warn!(target: "xylitol::session", "{msg}");
    } else if *warn_count == 4 {
        log::warn!(target: "xylitol::session", "...");
    }
}

/// Reject sessions whose header is missing or not the current [`SESSION_VERSION`].
pub fn enforce_session_version(entries: &[SessionEntry]) -> Result<(), XySessionStoreError> {
    let version = entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.version),
        _ => None,
    });
    match version {
        Some(v) if v == SESSION_VERSION => Ok(()),
        Some(v) => Err(XySessionStoreError::validation(
            unsupported_session_version_msg(v),
        )),
        None => Err(XySessionStoreError::validation(
            "session has no header entry",
        )),
    }
}

/// Validate the one legacy format that may be lazily migrated to the current format.
pub fn enforce_legacy_session_version(
    entries: &[SessionEntry],
) -> Result<(), XySessionStoreError> {
    let version = entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.version),
        _ => None,
    });
    match version {
        Some(v) if v == LEGACY_SESSION_VERSION => Ok(()),
        Some(v) => Err(XySessionStoreError::validation(format!(
            "session header version {v} cannot be migrated (only v{LEGACY_SESSION_VERSION} is supported)"
        ))),
        None => Err(XySessionStoreError::validation(
            "session has no header entry",
        )),
    }
}

fn unsupported_session_version_msg(v: u32) -> String {
    format!(
        "session header version {v} is not supported (require {SESSION_VERSION}); refusing legacy migrate"
    )
}
