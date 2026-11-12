use std::collections::HashMap;

use serde_json::json;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::layout::{ANCHOR_EVERY, Anchor, IndexState, LOG_FORMAT, Manifest, RotationPolicy};
use super::{LogSessionStore, StoreMode};
use crate::sync::{lock_rwlock_read, lock_rwlock_write};
use crate::types::{EntryBase, SESSION_VERSION, SessionEntry, SessionHeader};
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::message::now_ms;

pub(crate) const COMPAT_MESSAGE: &str =
    "session 格式已升级且不兼容，请开新 session；旧数据可用导出工具抢救（后续提供）";

fn now_secs() -> u64 {
    now_ms() / 1000
}

pub(crate) fn ensure_supported(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<(), XySessionStoreError> {
    let dir = store.layout.session_dir(session_id);
    if store.layout.legacy_path(session_id).exists() && !dir.exists() {
        return Err(XySessionStoreError::unsupported(COMPAT_MESSAGE));
    }
    if dir.exists()
        && !store.layout.log_dir(session_id).exists()
        && (store.layout.manifest_path(session_id).exists()
            || dir.join("segments").is_dir()
            || has_active_segments(&dir))
    {
        return Err(XySessionStoreError::unsupported(COMPAT_MESSAGE));
    }
    Ok(())
}

pub(crate) fn base_mut(entry: &mut SessionEntry) -> Option<&mut EntryBase> {
    match entry {
        SessionEntry::Header(_) => None,
        SessionEntry::Message(e) => Some(&mut e.base),
        SessionEntry::Compaction(e) => Some(&mut e.base),
        SessionEntry::BranchSummary(e) => Some(&mut e.base),
        SessionEntry::ModelChange(e) => Some(&mut e.base),
        SessionEntry::ThinkingLevelChange(e) => Some(&mut e.base),
        SessionEntry::Custom(e) => Some(&mut e.base),
        SessionEntry::CustomMessage(e) => Some(&mut e.base),
        SessionEntry::Label(e) => Some(&mut e.base),
        SessionEntry::SessionInfo(e) => Some(&mut e.base),
    }
}

fn prepare(entry: &mut SessionEntry, parent: Option<String>, with_id: bool) {
    let Some(base) = base_mut(entry) else {
        return;
    };
    if !with_id {
        base.id = Uuid::new_v4().to_string();
        base.parent_id = parent;
        base.timestamp = now_ms();
        return;
    }
    if base.id.is_empty() {
        base.id = Uuid::new_v4().to_string();
    }
    if base.parent_id.is_none() {
        base.parent_id = parent;
    }
    if base.timestamp == 0 {
        base.timestamp = now_ms();
    }
}

fn entry_line(entry: &SessionEntry, seq: u64, header: bool) -> Result<String, XySessionStoreError> {
    let mut value = serde_json::to_value(entry).map_err(XySessionStoreError::from)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("seq".into(), json!(seq));
        if header {
            obj.insert("format".into(), json!(LOG_FORMAT));
        }
    }
    serde_json::to_string(&value).map_err(XySessionStoreError::from)
}

fn is_turn_end(entry: &SessionEntry) -> bool {
    match entry {
        SessionEntry::Compaction(_) => true,
        SessionEntry::Message(m) => {
            let role = m.message.get("role").and_then(|v| v.as_str());
            match role {
                Some("assistant") => xylitol_plugin_sdk::session::count_tool_calls(&m.message) == 0,
                Some("user") => true,
                _ => false,
            }
        }
        _ => false,
    }
}

pub(crate) async fn create(
    store: &LogSessionStore,
    session_id: &str,
    cwd: Option<&str>,
    parent: Option<&str>,
) -> Result<(), XySessionStoreError> {
    create_with_header(
        store,
        session_id,
        header_entry(session_id, cwd.unwrap_or("."), parent),
    )
    .await
}

pub(crate) async fn create_with_fork(
    store: &LogSessionStore,
    session_id: &str,
    cwd: Option<&str>,
    parent: Option<&str>,
    at_entry_id: &str,
) -> Result<(), XySessionStoreError> {
    let mut header = header_entry(session_id, cwd.unwrap_or("."), parent);
    if let SessionEntry::Header(header) = &mut header {
        header.fork_at_entry_id = Some(at_entry_id.to_string());
    }
    create_with_header(store, session_id, header).await
}

async fn create_with_header(
    store: &LogSessionStore,
    session_id: &str,
    header: SessionEntry,
) -> Result<(), XySessionStoreError> {
    if matches!(store.mode, StoreMode::InMemory) {
        let mut memory = lock_rwlock_write(&store.memory);
        let session = memory.entry(session_id.to_string()).or_default();
        if session.entries.is_empty() {
            session.entries.push(header);
        }
        return Ok(());
    }

    ensure_supported(store, session_id)?;
    let log_dir = store.layout.log_dir(session_id);
    let manifest_path = store.layout.manifest_path(session_id);
    if log_dir.exists() {
        let files = list_log_files(store, session_id).await?;
        if !files.is_empty() {
            if !manifest_path.exists() {
                let manifest = rebuild_manifest(store, session_id).await?;
                write_manifest(store, session_id, &manifest).await?;
            }
            return Ok(());
        }
    }

    store.acquire_write_lock(session_id)?;
    tokio::fs::create_dir_all(&log_dir)
        .await
        .map_err(|e| XySessionStoreError::io("create session log dir", e))?;
    let line = entry_line(&header, 1, true)?;
    let path = store.layout.log_file(session_id, 1);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| XySessionStoreError::io("create session log file", e))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|e| XySessionStoreError::io("write session header", e))?;
    file.write_all(b"\n")
        .await
        .map_err(|e| XySessionStoreError::io("write session header", e))?;
    file.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync session header", e))?;
    drop(file);
    let manifest = Manifest {
        format: LOG_FORMAT.into(),
        session_id: session_id.into(),
        active_file: 1,
        tail_bytes: line.len() as u64 + 1,
        line_count: 1,
        next_seq: 2,
        leaf_id: None,
        rotation: RotationPolicy::default(),
        index: IndexState {
            anchor_every: ANCHOR_EVERY,
            built_at: now_secs(),
            files: vec![1],
        },
        updated_at: now_secs(),
    };
    write_manifest(store, session_id, &manifest).await?;
    write_index_file(
        store,
        session_id,
        1,
        &[Anchor {
            seq: 1,
            entry_id: session_id.into(),
            offset: 0,
        }],
    )
    .await?;
    Ok(())
}

fn header_entry(session_id: &str, cwd: &str, parent: Option<&str>) -> SessionEntry {
    SessionEntry::Header(SessionHeader {
        entry_type: "session".into(),
        version: SESSION_VERSION,
        id: session_id.into(),
        timestamp: now_ms(),
        cwd: cwd.into(),
        parent_session: parent.map(str::to_string),
        fork_at_entry_id: None,
        agent_profile: None,
        agent_profile_fingerprint: None,
    })
}

pub(crate) async fn append(
    store: &LogSessionStore,
    session_id: &str,
    entry: &SessionEntry,
    with_id: bool,
) -> Result<(), XySessionStoreError> {
    match store.mode {
        StoreMode::InMemory => append_memory(store, session_id, entry, with_id),
        StoreMode::Persisted => append_persisted(store, session_id, entry, with_id).await,
    }
}

fn append_memory(
    store: &LogSessionStore,
    session_id: &str,
    entry: &SessionEntry,
    with_id: bool,
) -> Result<(), XySessionStoreError> {
    let parent = store.get_leaf(session_id);
    let mut prepared = entry.clone();
    prepare(&mut prepared, parent, with_id);
    let entry_id = prepared.entry_id().map(str::to_string);
    let mut memory = lock_rwlock_write(&store.memory);
    let session = memory.entry(session_id.to_string()).or_default();
    if session.entries.is_empty() {
        session
            .entries
            .push(header_entry(session_id, ".", session.parent.as_deref()));
    }
    session.entries.push(prepared);
    drop(memory);
    if let Some(entry_id) = entry_id {
        store.set_leaf(session_id, Some(entry_id));
    }
    Ok(())
}

async fn append_persisted(
    store: &LogSessionStore,
    session_id: &str,
    entry: &SessionEntry,
    with_id: bool,
) -> Result<(), XySessionStoreError> {
    ensure_supported(store, session_id)?;
    if !store.layout.log_dir(session_id).exists()
        || !store.layout.manifest_path(session_id).exists()
    {
        create(store, session_id, None, None).await?;
    }
    store.acquire_write_lock(session_id)?;
    let mut manifest = read_manifest_or_rebuild(store, session_id).await?;
    let active_path = store.layout.log_file(session_id, manifest.active_file);
    if let Ok(metadata) = tokio::fs::metadata(&active_path).await
        && metadata.len() != manifest.tail_bytes
    {
        store.drop_cached_manifest(session_id);
        manifest = rebuild_manifest(store, session_id).await?;
    }
    let parent = manifest
        .leaf_id
        .clone()
        .or_else(|| store.get_leaf(session_id));
    let mut prepared = entry.clone();
    prepare(&mut prepared, parent, with_id);
    let seq = manifest.next_seq;
    let is_header = matches!(prepared, SessionEntry::Header(_));
    let turn_end = is_turn_end(&prepared);
    let line = entry_line(&prepared, seq, is_header)?;
    let file_no = manifest.active_file;
    let path = store.layout.log_file(session_id, file_no);
    let offset = manifest.tail_bytes;
    {
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|e| XySessionStoreError::io("open session log", e))?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| XySessionStoreError::io("write session entry", e))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| XySessionStoreError::io("write session entry", e))?;
        file.flush()
            .await
            .map_err(|e| XySessionStoreError::io("flush session entry", e))?;
        if turn_end {
            file.sync_all()
                .await
                .map_err(|e| XySessionStoreError::io("sync session entry", e))?;
        }
    }
    manifest.next_seq = seq + 1;
    manifest.tail_bytes = offset + line.len() as u64 + 1;
    manifest.line_count += 1;
    manifest.leaf_id = prepared.entry_id().map(str::to_string);
    manifest.updated_at = now_secs();
    let mut flush_manifest = turn_end;
    if manifest.line_count % ANCHOR_EVERY == 1 {
        let anchor = Anchor {
            seq: manifest.line_count,
            entry_id: manifest.leaf_id.clone().unwrap_or_default(),
            offset,
        };
        append_index_anchor(store, session_id, file_no, &anchor).await?;
        flush_manifest = true;
    }
    if matches!(prepared, SessionEntry::Compaction(_)) {
        rotate_at_compaction(store, session_id, &mut manifest, &line, &prepared).await?;
        flush_manifest = true;
    } else if turn_end && manifest.tail_bytes >= manifest.rotation.threshold_bytes {
        rotate_plain(store, session_id, &mut manifest).await?;
        flush_manifest = true;
    }
    if flush_manifest {
        write_manifest(store, session_id, &manifest).await?;
    } else {
        store.cache_manifest(session_id, manifest.clone(), true);
    }
    if let Some(entry_id) = prepared.entry_id() {
        store.set_leaf(session_id, Some(entry_id.to_string()));
    }
    Ok(())
}

pub(crate) async fn commit_compaction(
    store: &LogSessionStore,
    session_id: &str,
    entry: &SessionEntry,
) -> Result<(), XySessionStoreError> {
    if !matches!(entry, SessionEntry::Compaction(_)) {
        return Err(XySessionStoreError::validation(
            "compaction commit requires a compaction entry",
        ));
    }
    append(store, session_id, entry, false).await
}

async fn rotate_plain(
    store: &LogSessionStore,
    session_id: &str,
    manifest: &mut Manifest,
) -> Result<(), XySessionStoreError> {
    let new_file = manifest.active_file + 1;
    let path = store.layout.log_file(session_id, new_file);
    let file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| XySessionStoreError::io("create rotated session log", e))?;
    file.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync rotated session log", e))?;
    drop(file);
    manifest.active_file = new_file;
    manifest.tail_bytes = 0;
    manifest.line_count = 0;
    manifest.index.files.push(new_file);
    manifest.index.built_at = now_secs();
    manifest.updated_at = now_secs();
    Ok(())
}

async fn rotate_at_compaction(
    store: &LogSessionStore,
    session_id: &str,
    manifest: &mut Manifest,
    compaction_line: &str,
    compaction: &SessionEntry,
) -> Result<(), XySessionStoreError> {
    let SessionEntry::Compaction(compaction_entry) = compaction else {
        return Ok(());
    };
    let first_kept = compaction_entry.first_kept_entry_id.clone();
    let mut kept_ids = Vec::new();
    let raw = read_all_raw(store, session_id).await?;
    let mut current = compaction_entry.base.parent_id.clone();
    while let Some(id) = current {
        if id == first_kept {
            kept_ids.push(id);
            break;
        }
        let Some(line) = raw.get(&id) else {
            break;
        };
        kept_ids.push(id.clone());
        current = line.entry.parent_id().map(str::to_string);
    }
    kept_ids.reverse();

    let new_file = manifest.active_file + 1;
    let path = store.layout.log_file(session_id, new_file);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| XySessionStoreError::io("create compaction log", e))?;
    let mut anchors = Vec::new();
    let mut offset = 0u64;
    let mut line_count = 0u64;
    if line_count.is_multiple_of(ANCHOR_EVERY) {
        anchors.push(Anchor {
            seq: line_count + 1,
            entry_id: compaction.entry_id().unwrap_or_default().to_string(),
            offset,
        });
    }
    file.write_all(compaction_line.as_bytes())
        .await
        .map_err(|e| XySessionStoreError::io("write compaction line", e))?;
    file.write_all(b"\n")
        .await
        .map_err(|e| XySessionStoreError::io("write compaction line", e))?;
    offset += compaction_line.len() as u64 + 1;
    line_count += 1;
    for id in kept_ids {
        let Some(line) = raw.get(&id) else {
            continue;
        };
        if line_count.is_multiple_of(ANCHOR_EVERY) {
            anchors.push(Anchor {
                seq: line_count + 1,
                entry_id: id.clone(),
                offset,
            });
        }
        file.write_all(line.raw.as_bytes())
            .await
            .map_err(|e| XySessionStoreError::io("write kept-tail line", e))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| XySessionStoreError::io("write kept-tail line", e))?;
        offset += line.raw.len() as u64 + 1;
        line_count += 1;
    }
    file.flush()
        .await
        .map_err(|e| XySessionStoreError::io("flush compaction log", e))?;
    file.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync compaction log", e))?;
    drop(file);
    write_index_file(store, session_id, new_file, &anchors).await?;
    manifest.active_file = new_file;
    manifest.tail_bytes = offset;
    manifest.line_count = line_count;
    manifest.index.files.push(new_file);
    manifest.index.built_at = now_secs();
    manifest.updated_at = now_secs();
    Ok(())
}

pub(crate) async fn read_manifest(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Manifest, XySessionStoreError> {
    if let Some(cached) = lock_rwlock_read(&store.manifest_cache).get(session_id) {
        if cached.dirty {
            log::debug!(
                target: "xylitol::session",
                "session {session_id} manifest is dirty in memory; disk copy is behind"
            );
        }
        return Ok(cached.manifest.clone());
    }
    let raw = tokio::fs::read(store.layout.manifest_path(session_id))
        .await
        .map_err(|e| XySessionStoreError::io("read session manifest", e))?;
    serde_json::from_slice(&raw)
        .map_err(|e| XySessionStoreError::validation(format!("parse session manifest: {e}")))
}

pub(crate) async fn read_manifest_or_rebuild(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Manifest, XySessionStoreError> {
    if !store.layout.manifest_path(session_id).exists() {
        let manifest = rebuild_manifest(store, session_id).await?;
        write_manifest(store, session_id, &manifest).await?;
        return Ok(manifest);
    }
    if let Ok(manifest) = read_manifest(store, session_id).await
        && manifest.format == LOG_FORMAT
        && store
            .layout
            .log_file(session_id, manifest.active_file)
            .exists()
    {
        return Ok(manifest);
    }
    let manifest = rebuild_manifest(store, session_id).await?;
    write_manifest(store, session_id, &manifest).await?;
    Ok(manifest)
}

pub(crate) async fn rebuild_manifest(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Manifest, XySessionStoreError> {
    let files = list_log_files(store, session_id).await?;
    let Some(active_file) = files.last().copied() else {
        return Err(XySessionStoreError::not_found(session_id));
    };
    let raw = read_all_raw(store, session_id).await?;
    let mut max_seq = 1u64;
    let mut header_format: Option<String> = None;
    let mut leaf_id: Option<String> = None;
    let mut leaf_seq = 0u64;
    for line in raw.values() {
        max_seq = max_seq.max(line.seq);
        if matches!(line.entry, SessionEntry::Header(_)) {
            header_format = line.format.clone();
        }
        if !matches!(line.entry, SessionEntry::Header(_)) && line.seq >= leaf_seq {
            leaf_seq = line.seq;
            leaf_id = line.entry.entry_id().map(str::to_string);
        }
    }
    if header_format.as_deref() != Some(LOG_FORMAT) {
        return Err(XySessionStoreError::unsupported(COMPAT_MESSAGE));
    }
    let active_path = store.layout.log_file(session_id, active_file);
    let tail_bytes = tokio::fs::metadata(&active_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let line_count = read_file_lines(store, session_id, active_file).await?.len() as u64;
    Ok(Manifest {
        format: LOG_FORMAT.into(),
        session_id: session_id.into(),
        active_file,
        tail_bytes,
        line_count,
        next_seq: max_seq + 1,
        leaf_id,
        rotation: RotationPolicy::default(),
        index: IndexState {
            anchor_every: ANCHOR_EVERY,
            built_at: now_secs(),
            files,
        },
        updated_at: now_secs(),
    })
}

pub(crate) async fn write_manifest(
    store: &LogSessionStore,
    session_id: &str,
    manifest: &Manifest,
) -> Result<(), XySessionStoreError> {
    let path = store.layout.manifest_path(session_id);
    let bytes = serde_json::to_vec(manifest).map_err(XySessionStoreError::from)?;
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| XySessionStoreError::io("create session manifest", e))?;
    file.write_all(&bytes)
        .await
        .map_err(|e| XySessionStoreError::io("write session manifest", e))?;
    file.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync session manifest", e))?;
    store.cache_manifest(session_id, manifest.clone(), false);
    Ok(())
}

pub(crate) async fn list_log_files(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Vec<u32>, XySessionStoreError> {
    let dir = store.layout.log_dir(session_id);
    let mut read = match tokio::fs::read_dir(&dir).await {
        Ok(read) => read,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(XySessionStoreError::io("read session log dir", error)),
    };
    let mut files = Vec::new();
    while let Some(entry) = read
        .next_entry()
        .await
        .map_err(|e| XySessionStoreError::io("read session log dir entry", e))?
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".jsonl")
            && let Ok(file) = stem.parse::<u32>()
        {
            files.push(file);
        }
    }
    files.sort_unstable();
    Ok(files)
}

#[derive(Debug, Clone)]
pub(crate) struct RawLine {
    pub(crate) seq: u64,
    pub(crate) entry: SessionEntry,
    pub(crate) raw: String,
    pub(crate) format: Option<String>,
}

pub(crate) async fn read_file_lines(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
) -> Result<Vec<RawLine>, XySessionStoreError> {
    let path = store.layout.log_file(session_id, file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(XySessionStoreError::io("read session log", error)),
    };
    let mut out = Vec::new();
    let mut warn_count = 0usize;
    let text = String::from_utf8_lossy(&bytes);
    let complete = bytes.ends_with(b"\n");
    let parts: Vec<&str> = text.split('\n').collect();
    let last_index = parts.len().saturating_sub(1);
    for (index, line) in parts.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if !complete && index == last_index {
            break;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            warn_count += 1;
            if warn_count <= 3 {
                log::warn!(target: "xylitol::session", "skip unparsable session log line");
            }
            continue;
        };
        let seq = value.get("seq").and_then(|v| v.as_u64()).unwrap_or(1);
        let format = value
            .get("format")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let Ok(entry) = serde_json::from_value::<SessionEntry>(value) else {
            warn_count += 1;
            if warn_count <= 3 {
                log::warn!(target: "xylitol::session", "skip unparsable session log entry");
            }
            continue;
        };
        out.push(RawLine {
            seq,
            entry,
            raw: line.to_string(),
            format,
        });
    }
    Ok(out)
}

pub(crate) async fn read_all_raw(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<HashMap<String, RawLine>, XySessionStoreError> {
    let files = list_log_files(store, session_id).await?;
    let mut map = HashMap::new();
    for file in files {
        for line in read_file_lines(store, session_id, file).await? {
            let key = match &line.entry {
                SessionEntry::Header(header) => format!("__header:{}", header.id),
                _ => match line.entry.entry_id() {
                    Some(entry_id) => entry_id.to_string(),
                    None => continue,
                },
            };
            map.insert(key, line);
        }
    }
    Ok(map)
}

pub(crate) async fn write_index_file(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
    anchors: &[Anchor],
) -> Result<(), XySessionStoreError> {
    tokio::fs::create_dir_all(store.layout.index_dir(session_id))
        .await
        .map_err(|e| XySessionStoreError::io("create session index dir", e))?;
    let mut content = String::new();
    for anchor in anchors {
        content.push_str(&serde_json::to_string(anchor).map_err(XySessionStoreError::from)?);
        content.push('\n');
    }
    let mut handle = tokio::fs::File::create(store.layout.index_path(session_id, file))
        .await
        .map_err(|e| XySessionStoreError::io("create session index", e))?;
    handle
        .write_all(content.as_bytes())
        .await
        .map_err(|e| XySessionStoreError::io("write session index", e))?;
    handle
        .sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync session index", e))?;
    Ok(())
}

async fn append_index_anchor(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
    anchor: &Anchor,
) -> Result<(), XySessionStoreError> {
    tokio::fs::create_dir_all(store.layout.index_dir(session_id))
        .await
        .map_err(|e| XySessionStoreError::io("create session index dir", e))?;
    let line = serde_json::to_string(anchor).map_err(XySessionStoreError::from)?;
    let mut handle = tokio::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(store.layout.index_path(session_id, file))
        .await
        .map_err(|e| XySessionStoreError::io("open session index", e))?;
    handle
        .write_all(format!("{line}\n").as_bytes())
        .await
        .map_err(|e| XySessionStoreError::io("write session index anchor", e))?;
    handle
        .flush()
        .await
        .map_err(|e| XySessionStoreError::io("flush session index anchor", e))?;
    Ok(())
}

pub(crate) async fn truncate_partial_tail(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<bool, XySessionStoreError> {
    let manifest = read_manifest_or_rebuild(store, session_id).await?;
    let path = store.layout.log_file(session_id, manifest.active_file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(XySessionStoreError::io("read session log", error)),
    };
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        return Ok(false);
    }
    let cut = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .await
        .map_err(|e| XySessionStoreError::io("open session log for truncate", e))?;
    file.set_len(cut as u64)
        .await
        .map_err(|e| XySessionStoreError::io("truncate session log", e))?;
    file.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync truncated session log", e))?;
    Ok(true)
}

#[allow(dead_code)]
pub(crate) async fn read_index(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
) -> Result<Vec<Anchor>, XySessionStoreError> {
    let path = store.layout.index_path(session_id, file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(XySessionStoreError::io("read session index", error)),
    };
    let mut anchors = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(anchor) = serde_json::from_str::<Anchor>(line) {
            anchors.push(anchor);
        }
    }
    Ok(anchors)
}

fn has_active_segments(dir: &std::path::Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("active-") && name.ends_with(".jsonl") {
            return true;
        }
    }
    false
}
