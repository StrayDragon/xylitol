use std::collections::{HashMap, HashSet};

use super::append::{
    COMPAT_MESSAGE, RawLine, ensure_supported, list_log_files, read_file_lines, read_manifest,
};
use super::layout::LOG_FORMAT;
use super::{LogSessionStore, StoreMode};
use crate::sync::{lock_rwlock_read, lock_rwlock_write};
use crate::types::{SESSION_VERSION, SessionContext, SessionEntry};
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::{
    EdgeKind, EdgeRecord, build_context, done_bash_ids, transcript_leaf_anchor,
};

pub(crate) async fn load_all(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    match store.mode {
        StoreMode::InMemory => memory_load_all(store, session_id),
        StoreMode::Persisted => persisted_load_all(store, session_id, 0).await,
    }
}

fn memory_load_all(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let memory = lock_rwlock_read(&store.memory);
    let mut chain: Vec<Vec<SessionEntry>> = Vec::new();
    let mut current = Some(session_id.to_string());
    let mut seen = HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            break;
        }
        let Some(session) = memory.get(&id) else {
            break;
        };
        chain.push(session.entries.clone());
        current = session.parent.clone();
    }
    chain.reverse();
    Ok(chain.into_iter().flatten().collect())
}

async fn persisted_load_all(
    store: &LogSessionStore,
    session_id: &str,
    depth: usize,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    if depth > 32 {
        return Err(XySessionStoreError::validation(
            "session parent chain is too deep",
        ));
    }
    if depth == 0 {
        super::maintain::quick_check(store, session_id).await?;
    }
    ensure_supported(store, session_id)?;
    if !store.layout.log_dir(session_id).exists() {
        return Err(XySessionStoreError::not_found(session_id));
    }
    let files = list_log_files(store, session_id).await?;
    if files.is_empty() {
        return Err(XySessionStoreError::not_found(session_id));
    }
    let mut map: HashMap<String, RawLine> = HashMap::new();
    let mut header_line: Option<RawLine> = None;
    let mut header_format: Option<String> = None;
    let mut header_version: Option<u32> = None;
    for file in files {
        for line in read_file_lines(store, session_id, file).await? {
            if let SessionEntry::Header(header) = &line.entry {
                header_format = line.format.clone();
                header_version = Some(header.version);
                header_line = Some(line.clone());
                continue;
            }
            if let Some(entry_id) = line.entry.entry_id() {
                map.insert(entry_id.to_string(), line);
            }
        }
    }
    if header_format.as_deref() != Some(LOG_FORMAT) {
        return Err(XySessionStoreError::unsupported(COMPAT_MESSAGE));
    }
    if header_version != Some(SESSION_VERSION) {
        return Err(XySessionStoreError::validation(format!(
            "session header version {:?} is not supported (require {SESSION_VERSION})",
            header_version
        )));
    }
    let mut lines: Vec<RawLine> = map.into_values().collect();
    lines.sort_by_key(|line| line.seq);
    let mut own: Vec<SessionEntry> = Vec::with_capacity(lines.len() + 1);
    if let Some(header) = header_line {
        own.push(header.entry);
    }
    own.extend(lines.into_iter().map(|line| line.entry));
    let parent = own.iter().find_map(|entry| match entry {
        SessionEntry::Header(header) => header.parent_session.clone(),
        _ => None,
    });
    let Some(parent_id) = parent else {
        return Ok(own);
    };
    let parent_entries = Box::pin(persisted_load_all(store, &parent_id, depth + 1)).await?;
    let mut merged = parent_entries;
    for entry in own {
        if let Some(entry_id) = entry.entry_id() {
            if let Some(position) = merged
                .iter()
                .position(|existing| existing.entry_id() == Some(entry_id))
            {
                merged[position] = entry;
            } else {
                merged.push(entry);
            }
        } else {
            merged.push(entry);
        }
    }
    Ok(merged)
}

async fn resolve_leaf(
    store: &LogSessionStore,
    session_id: &str,
    entries: &[SessionEntry],
    hint: Option<&str>,
) -> Option<String> {
    if let Some(hint) = hint {
        return Some(hint.to_string());
    }
    if let Some(leaf) = store.get_leaf(session_id) {
        return Some(leaf);
    }
    if matches!(store.mode, StoreMode::Persisted)
        && let Ok(manifest) = read_manifest(store, session_id).await
        && let Some(leaf) = manifest.leaf_id
    {
        return Some(leaf);
    }
    transcript_leaf_anchor(entries, None).or_else(|| {
        entries
            .iter()
            .rev()
            .find_map(|entry| entry.entry_id().map(str::to_string))
    })
}

pub(crate) async fn load_branch(
    store: &LogSessionStore,
    session_id: &str,
    hint: Option<&str>,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let entries = load_all(store, session_id).await?;
    let map: HashMap<&str, &SessionEntry> = entries
        .iter()
        .filter_map(|entry| entry.entry_id().map(|id| (id, entry)))
        .collect();
    let Some(leaf) = resolve_leaf(store, session_id, &entries, hint).await else {
        return Ok(Vec::new());
    };
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let mut cut: Option<String> = None;
    let mut current = Some(leaf.clone());
    while let Some(id) = current {
        if !visited.insert(id.clone()) {
            break;
        }
        let Some(entry) = map.get(id.as_str()) else {
            break;
        };
        path.push((*entry).clone());
        if let SessionEntry::Compaction(compaction) = entry {
            cut = Some(compaction.first_kept_entry_id.clone());
        }
        if cut.as_deref() == Some(id.as_str()) {
            break;
        }
        current = entry.parent_id().map(str::to_string);
    }
    path.reverse();
    store.set_leaf(session_id, Some(leaf));
    Ok(path)
}

pub(crate) async fn build_context_entries(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let entries = load_all(store, session_id).await?;
    let Some(head) = resolve_leaf(store, session_id, &entries, None).await else {
        return Ok(Vec::new());
    };
    let edges = read_edges(store, session_id, &entries).await?;
    let done = done_bash_ids(&entries);
    let view = build_context(&head, &entries, &edges, &done)
        .map_err(|error| XySessionStoreError::validation(format!("session graph: {error}")))?;
    Ok(view.nodes)
}

pub(crate) async fn build_session_context(
    store: &LogSessionStore,
    session_id: &str,
) -> Result<SessionContext, XySessionStoreError> {
    let entries = load_all(store, session_id).await?;
    let head = resolve_leaf(store, session_id, &entries, None).await;
    let mut thinking_level = String::from("off");
    let mut model: Option<(String, String)> = None;
    for entry in &entries {
        match entry {
            SessionEntry::ModelChange(change) => {
                model = Some((change.provider.clone(), change.model_id.clone()));
            }
            SessionEntry::ThinkingLevelChange(change) => {
                thinking_level = change.thinking_level.clone();
            }
            _ => {}
        }
    }
    let messages = match head {
        Some(head) => {
            let edges = read_edges(store, session_id, &entries).await?;
            let done = done_bash_ids(&entries);
            let view = build_context(&head, &entries, &edges, &done).map_err(|error| {
                XySessionStoreError::validation(format!("session graph: {error}"))
            })?;
            view.messages
                .iter()
                .filter_map(|message| serde_json::to_value(message).ok())
                .collect()
        }
        None => Vec::new(),
    };
    Ok(SessionContext {
        messages,
        thinking_level,
        model,
    })
}

fn derived_parent_edges(entries: &[SessionEntry]) -> Vec<EdgeRecord> {
    entries
        .iter()
        .filter_map(|entry| {
            let id = entry.entry_id()?;
            let parent = entry.parent_id()?;
            Some(EdgeRecord {
                id: format!("parent:{id}"),
                edge_kind: EdgeKind::Parent,
                from: parent.to_string(),
                to: id.to_string(),
                order: 0,
                enabled: true,
                timestamp: entry.base().map(|base| base.timestamp).unwrap_or(0),
            })
        })
        .collect()
}

pub(crate) async fn read_edges(
    store: &LogSessionStore,
    session_id: &str,
    entries: &[SessionEntry],
) -> Result<Vec<EdgeRecord>, XySessionStoreError> {
    let mut edges = derived_parent_edges(entries);
    match store.mode {
        StoreMode::Persisted => {
            let path = store.layout.graph_path(session_id);
            if path.exists() {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| XySessionStoreError::io("read session graph", e))?;
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<EdgeRecord>(line) {
                        Ok(edge) => edges.push(edge),
                        Err(error) => log::warn!(
                            target: "xylitol::session",
                            "skip session graph edge line: {error}"
                        ),
                    }
                }
            }
        }
        StoreMode::InMemory => {
            let memory = lock_rwlock_read(&store.memory);
            if let Some(session) = memory.get(session_id) {
                edges.extend(session.edges.iter().cloned());
            }
        }
    }
    Ok(edges)
}

pub(crate) async fn append_edge(
    store: &LogSessionStore,
    session_id: &str,
    edge: &EdgeRecord,
) -> Result<(), XySessionStoreError> {
    match store.mode {
        StoreMode::Persisted => {
            store.acquire_write_lock(session_id)?;
            let path = store.layout.graph_path(session_id);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| XySessionStoreError::io("create session graph dir", e))?;
            }
            let line = serde_json::to_string(edge).map_err(XySessionStoreError::from)?;
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| XySessionStoreError::io("open session graph", e))?;
            file.write_all(format!("{line}\n").as_bytes())
                .await
                .map_err(|e| XySessionStoreError::io("write session graph", e))?;
            file.flush()
                .await
                .map_err(|e| XySessionStoreError::io("flush session graph", e))?;
        }
        StoreMode::InMemory => {
            lock_rwlock_write(&store.memory)
                .entry(session_id.to_string())
                .or_default()
                .edges
                .push(edge.clone());
        }
    }
    Ok(())
}

pub(crate) async fn session_display_name_from_entries(entries: &[SessionEntry]) -> Option<String> {
    for entry in entries.iter().rev() {
        if let SessionEntry::SessionInfo(info) = entry {
            return info.name.clone().filter(|name| !name.is_empty());
        }
    }
    None
}

pub(crate) async fn list_session_ids(
    store: &LogSessionStore,
) -> Result<Vec<String>, XySessionStoreError> {
    match store.mode {
        StoreMode::InMemory => {
            let memory = lock_rwlock_read(&store.memory);
            Ok(memory.keys().cloned().collect())
        }
        StoreMode::Persisted => {
            let dir = store.layout.sessions_dir.clone();
            let mut read = match tokio::fs::read_dir(&dir).await {
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Vec::new());
                }
                Err(error) => return Err(XySessionStoreError::io("read sessions dir", error)),
            };
            let mut files: Vec<(String, Option<std::time::SystemTime>)> = Vec::new();
            while let Some(entry) = read
                .next_entry()
                .await
                .map_err(|e| XySessionStoreError::io("read dir entry", e))?
            {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry
                    .file_type()
                    .await
                    .map(|ty| ty.is_dir())
                    .unwrap_or(false);
                if is_dir && entry.path().join("log").is_dir() {
                    let modified = entry
                        .metadata()
                        .await
                        .map(|meta| meta.modified().ok())
                        .ok()
                        .flatten();
                    files.push((name, modified));
                }
            }
            files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
            Ok(files.into_iter().map(|(id, _)| id).collect())
        }
    }
}
