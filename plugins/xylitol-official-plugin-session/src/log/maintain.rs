#![allow(dead_code)]

use super::append::{
    ensure_supported, list_log_files, read_manifest_or_rebuild, repair_partial_tail,
    write_index_file,
};
use super::layout::{
    ANCHOR_EVERY, Anchor, INITIAL_THRESHOLD_BYTES, MAX_THRESHOLD_BYTES, MIN_THRESHOLD_BYTES,
};
use super::project::{list_session_ids, load_all};
use super::{LogSessionStore, StoreMode};
use crate::types::SessionEntry;
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::is_user_message;

pub(crate) struct SessionHealth {
    pub(crate) id: String,
    pub(crate) files: usize,
    pub(crate) bytes: u64,
    pub(crate) entries: usize,
    pub(crate) suggested_threshold_bytes: u64,
}

pub(crate) struct MaintainReport {
    pub(crate) sessions: Vec<SessionHealth>,
}

pub(crate) async fn quick_check(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<(), XySessionStoreError> {
    if matches!(store.mode, StoreMode::InMemory) {
        return Ok(());
    }
    ensure_supported(store, session_id)?;
    if !store.layout.log_dir(session_id).exists() {
        return Err(XySessionStoreError::not_found(session_id));
    }
    remove_tmp_files(store, session_id).await?;
    let manifest = read_manifest_or_rebuild(store, session_id).await?;
    for file in manifest.index.files {
        if !store.layout.index_path(session_id, file).exists() {
            rebuild_index(store, session_id, file).await?;
        }
    }
    Ok(())
}

async fn remove_tmp_files(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<(), XySessionStoreError> {
    for dir in [
        store.layout.session_dir(session_id),
        store.layout.log_dir(session_id),
    ] {
        let mut read = match tokio::fs::read_dir(&dir).await {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(XySessionStoreError::io("read session dir", error)),
        };
        while let Some(entry) = read
            .next_entry()
            .await
            .map_err(|e| XySessionStoreError::io("read session dir entry", e))?
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".tmp") {
                tokio::fs::remove_file(entry.path())
                    .await
                    .map_err(|e| XySessionStoreError::io("remove session tmp file", e))?;
            }
        }
    }
    Ok(())
}

pub(crate) async fn rebuild_index(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
) -> Result<(), XySessionStoreError> {
    let path = store.layout.log_file(session_id, file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(XySessionStoreError::io("read session log", error)),
    };
    let mut anchors = Vec::new();
    let mut offset = 0u64;
    let mut line_no = 0u64;
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            offset += 1;
            continue;
        }
        if line_no.is_multiple_of(ANCHOR_EVERY) {
            let value: serde_json::Value = serde_json::from_slice(line).unwrap_or_default();
            let entry_id = value
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            anchors.push(Anchor {
                seq: line_no + 1,
                entry_id,
                offset,
            });
        }
        offset += line.len() as u64 + 1;
        line_no += 1;
    }
    write_index_file(store, session_id, file, &anchors).await
}

pub(crate) async fn maintain_session(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<SessionHealth, XySessionStoreError> {
    let already_held = store.holds_write_lock(session_id);
    let mut acquired = false;
    if !already_held {
        match store.acquire_write_lock(session_id) {
            Ok(()) => acquired = true,
            Err(XySessionStoreError::WriteHeld { .. }) => {}
            Err(error) => return Err(error),
        }
    }
    let result = maintain_session_locked(store, session_id, already_held || acquired).await;
    if acquired {
        store.release_write_lock(session_id);
    }
    result
}

async fn maintain_session_locked(
    store: &LogSessionStore,
    session_id: &str,
    repair: bool,
) -> Result<SessionHealth, XySessionStoreError> {
    if repair {
        repair_partial_tail(store, session_id).await?;
    }
    quick_check(store, session_id).await?;
    let files = list_log_files(store, session_id).await?;
    let mut bytes = 0u64;
    for file in &files {
        rebuild_index(store, session_id, *file).await?;
        bytes += tokio::fs::metadata(store.layout.log_file(session_id, *file))
            .await
            .map(|meta| meta.len())
            .unwrap_or(0);
    }
    let entries = load_all(store, session_id).await?;
    Ok(SessionHealth {
        id: session_id.into(),
        files: files.len(),
        bytes,
        entries: entries.len(),
        suggested_threshold_bytes: suggested_threshold(&entries),
    })
}

fn suggested_threshold(entries: &[SessionEntry]) -> u64 {
    let mut turn_bytes = Vec::new();
    let mut current = 0u64;
    for entry in entries {
        let size = serde_json::to_vec(entry)
            .map(|bytes| bytes.len() as u64)
            .unwrap_or(0);
        if is_user_message(entry) && current > 0 {
            turn_bytes.push(current);
            current = 0;
        }
        current += size;
    }
    if current > 0 {
        turn_bytes.push(current);
    }
    if turn_bytes.is_empty() {
        return INITIAL_THRESHOLD_BYTES;
    }
    turn_bytes.sort_unstable();
    let median = turn_bytes[turn_bytes.len() / 2];
    median
        .saturating_mul(256)
        .clamp(MIN_THRESHOLD_BYTES, MAX_THRESHOLD_BYTES)
}

pub(crate) async fn maintain_all(
    store: &LogSessionStore,
) -> Result<MaintainReport, XySessionStoreError> {
    let ids = list_session_ids(store).await?;
    let mut sessions = Vec::new();
    for id in ids {
        match maintain_session(store, &id).await {
            Ok(health) => sessions.push(health),
            Err(error) => log::warn!(
                target: "xylitol::session",
                "session maintenance skipped {id}: {error}"
            ),
        }
    }
    Ok(MaintainReport { sessions })
}
