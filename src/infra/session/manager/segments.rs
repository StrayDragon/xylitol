//! Segmented session layout primitives.
//!
//! This module owns filesystem vocabulary and crash-atomic file replacement.
//! Tree projection and compaction policy stay in their existing modules.

use std::path::{Component, Path, PathBuf};

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::SessionManager;
use crate::infra::session::types::SessionEntry;
use crate::protocol::error::XySessionStoreError;
use crate::protocol::session::{
    SESSION_VERSION, SessionManifest, SessionSegment, enforce_session_version,
    parse_session_jsonl_lines,
};

const INITIAL_GENERATION: u64 = 0;

impl SessionManager {
    pub(super) fn session_dir_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    pub(super) fn legacy_session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{session_id}.jsonl"))
    }

    pub(super) fn manifest_path(&self, session_id: &str) -> PathBuf {
        self.session_dir_path(session_id).join("manifest.json")
    }

    pub(super) fn segments_dir_path(&self, session_id: &str) -> PathBuf {
        self.session_dir_path(session_id).join("segments")
    }

    pub(super) fn current_active_path(&self, session_id: &str) -> PathBuf {
        let fallback = self
            .session_dir_path(session_id)
            .join(format!("active-{INITIAL_GENERATION}.jsonl"));
        let Ok(raw) = std::fs::read(self.manifest_path(session_id)) else {
            return fallback;
        };
        let Ok(manifest) = serde_json::from_slice::<SessionManifest>(&raw) else {
            return fallback;
        };
        self.resolve_segment_path(session_id, &manifest.active_segment.path)
            .unwrap_or(fallback)
    }

    pub(super) fn resolve_segment_path(
        &self,
        session_id: &str,
        relative_path: &str,
    ) -> Result<PathBuf, XySessionStoreError> {
        let relative = Path::new(relative_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(XySessionStoreError::validation(format!(
                "session manifest segment path must stay relative to session root: {relative_path}"
            )));
        }
        Ok(self.session_dir_path(session_id).join(relative))
    }

    pub(super) async fn read_manifest(
        &self,
        session_id: &str,
    ) -> Result<SessionManifest, XySessionStoreError> {
        let path = self.manifest_path(session_id);
        let raw = tokio::fs::read(&path)
            .await
            .map_err(|e| XySessionStoreError::io("read session manifest", e))?;
        let manifest: SessionManifest = serde_json::from_slice(&raw).map_err(|e| {
            XySessionStoreError::validation(format!("invalid session manifest: {e}"))
        })?;
        self.validate_manifest(session_id, &manifest)?;
        Ok(manifest)
    }

    pub(super) fn validate_manifest(
        &self,
        session_id: &str,
        manifest: &SessionManifest,
    ) -> Result<(), XySessionStoreError> {
        if manifest.format_version != SESSION_VERSION {
            return Err(XySessionStoreError::validation(format!(
                "session manifest formatVersion {} is not supported (require {SESSION_VERSION})",
                manifest.format_version
            )));
        }
        if manifest.session_id != session_id {
            return Err(XySessionStoreError::validation(format!(
                "session manifest id {:?} does not match requested session {session_id:?}",
                manifest.session_id
            )));
        }

        let mut paths = std::collections::HashSet::new();
        for segment in manifest
            .sealed_segments
            .iter()
            .chain(std::iter::once(&manifest.active_segment))
        {
            let path = self.resolve_segment_path(session_id, &segment.path)?;
            if !paths.insert(path) {
                return Err(XySessionStoreError::validation(
                    "session manifest references a segment more than once",
                ));
            }
        }
        Ok(())
    }

    pub(super) async fn read_segment(
        &self,
        session_id: &str,
        segment: &SessionSegment,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let path = self.resolve_segment_path(session_id, &segment.path)?;
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| XySessionStoreError::io("read session segment", e))?;
        let (entries, _) = parse_session_jsonl_lines(&content);
        Ok(entries)
    }

    pub(super) async fn load_entries(
        &self,
        session_id: &str,
        manifest: &SessionManifest,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let mut entries = Vec::new();
        for segment in &manifest.sealed_segments {
            entries.extend(self.read_segment(session_id, segment).await?);
        }
        entries.extend(
            self.read_segment(session_id, &manifest.active_segment)
                .await?,
        );
        enforce_session_version(&entries)?;
        Ok(entries)
    }

    pub(super) async fn write_segment_atomically(
        &self,
        session_id: &str,
        segment: &SessionSegment,
        entries: &[SessionEntry],
    ) -> Result<(), XySessionStoreError> {
        let path = self.resolve_segment_path(session_id, &segment.path)?;
        let content = serialize_entries(entries)?;
        write_file_atomically(&path, &content).await
    }

    pub(super) async fn write_manifest_atomically(
        &self,
        session_id: &str,
        manifest: &SessionManifest,
    ) -> Result<(), XySessionStoreError> {
        self.validate_manifest(session_id, manifest)?;
        let path = self.manifest_path(session_id);
        let content = serde_json::to_string_pretty(manifest)
            .map_err(|e| XySessionStoreError::validation(format!("serialize manifest: {e}")))?;
        write_file_atomically(&path, &format!("{content}\n")).await
    }

    pub(super) fn segment_descriptor(
        path: impl Into<String>,
        generation: u64,
        entries: &[SessionEntry],
    ) -> SessionSegment {
        SessionSegment {
            path: path.into(),
            generation,
            first_entry_id: entries
                .iter()
                .find_map(|entry| entry.entry_id().map(str::to_owned)),
            last_entry_id: entries
                .iter()
                .rev()
                .find_map(|entry| entry.entry_id().map(str::to_owned)),
            includes_header: entries
                .iter()
                .any(|entry| matches!(entry, SessionEntry::Header(_))),
        }
    }

    pub(super) fn next_generation(manifest: &SessionManifest) -> u64 {
        manifest
            .sealed_segments
            .iter()
            .map(|segment| segment.generation)
            .chain(std::iter::once(manifest.active_segment.generation))
            .max()
            .unwrap_or(INITIAL_GENERATION)
            .saturating_add(1)
    }

    pub(super) async fn commit_entries(
        &self,
        session_id: &str,
        entries: &[SessionEntry],
        generation: u64,
    ) -> Result<SessionManifest, XySessionStoreError> {
        let active_name = format!("active-{generation}.jsonl");
        let active = Self::segment_descriptor(active_name, generation, entries);
        let mut manifest = SessionManifest::new(session_id, active.clone());
        manifest.leaf_entry_id = entries
            .iter()
            .rev()
            .find_map(|entry| entry.entry_id().map(str::to_owned));
        self.write_segment_atomically(session_id, &active, entries)
            .await?;
        if let Err(error) = self.write_manifest_atomically(session_id, &manifest).await {
            let _ =
                tokio::fs::remove_file(self.resolve_segment_path(session_id, &active.path)?).await;
            return Err(error);
        }
        Ok(manifest)
    }

    pub(super) async fn replace_entries(
        &self,
        session_id: &str,
        entries: &[SessionEntry],
    ) -> Result<(), XySessionStoreError> {
        let old_manifest = if self.manifest_path(session_id).exists() {
            Some(self.read_manifest(session_id).await?)
        } else {
            None
        };
        let generation = old_manifest
            .as_ref()
            .map(Self::next_generation)
            .unwrap_or(INITIAL_GENERATION);
        let new_manifest = self.commit_entries(session_id, entries, generation).await?;

        if let Some(old) = old_manifest {
            let old_paths = old
                .sealed_segments
                .into_iter()
                .map(|segment| segment.path)
                .chain(std::iter::once(old.active_segment.path));
            for path in old_paths {
                if path != new_manifest.active_segment.path
                    && let Ok(path) = self.resolve_segment_path(session_id, &path)
                {
                    let _ = tokio::fs::remove_file(path).await;
                }
            }
            let _ = tokio::fs::remove_dir(self.segments_dir_path(session_id)).await;
        }
        Ok(())
    }

    pub(super) async fn migrate_legacy_session(
        &self,
        session_id: &str,
    ) -> Result<(), XySessionStoreError> {
        let legacy_path = self.legacy_session_path(session_id);
        let content = tokio::fs::read_to_string(&legacy_path)
            .await
            .map_err(|e| XySessionStoreError::io("read v6 session for migration", e))?;
        let entries = parse_legacy_entries(&content)?;
        crate::protocol::session::enforce_legacy_session_version(&entries)?;

        let mut migrated = entries;
        for entry in &mut migrated {
            match entry {
                SessionEntry::Header(header) => header.version = SESSION_VERSION,
                SessionEntry::Compaction(compaction) if compaction.policy.is_none() => {
                    compaction.policy =
                        Some(crate::protocol::session::CompactionPolicySnapshot::legacy_unknown());
                }
                _ => {}
            }
        }

        self.commit_entries(session_id, &migrated, INITIAL_GENERATION)
            .await?;
        // The manifest is the visible commit point. A failed cleanup is
        // harmless: the next access prefers the committed layout and retries
        // the removal.
        if let Err(error) = tokio::fs::remove_file(&legacy_path).await {
            log::warn!(
                target: "xylitol::session",
                "v6 session {session_id} migrated but legacy cleanup failed: {error}"
            );
        }
        Ok(())
    }
}

pub(super) fn serialize_entries(entries: &[SessionEntry]) -> Result<String, XySessionStoreError> {
    let mut content = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(XySessionStoreError::from)?;
        content.push_str(&line);
        content.push('\n');
    }
    Ok(content)
}

pub(super) async fn write_file_atomically(
    path: &Path,
    content: &str,
) -> Result<(), XySessionStoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| XySessionStoreError::validation("session file has no parent directory"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| XySessionStoreError::io("create session file directory", e))?;

    let tmp_path = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("session"),
        Uuid::new_v4()
    ));
    let result = async {
        let mut tmp = tokio::fs::File::create(&tmp_path)
            .await
            .map_err(|e| XySessionStoreError::io("create session tmp file", e))?;
        tmp.write_all(content.as_bytes())
            .await
            .map_err(|e| XySessionStoreError::io("write session tmp file", e))?;
        tmp.sync_all()
            .await
            .map_err(|e| XySessionStoreError::io("sync session tmp file", e))?;
        tokio::fs::rename(&tmp_path, path)
            .await
            .map_err(|e| XySessionStoreError::io("rename session file", e))?;
        sync_parent_dir(parent).await;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }
    result
}

async fn sync_parent_dir(parent: &Path) {
    if let Ok(dir) = tokio::fs::File::open(parent).await {
        let _ = dir.sync_all().await;
    }
}

fn parse_legacy_entries(content: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let mut entries = Vec::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry = serde_json::from_str::<SessionEntry>(line).map_err(|error| {
            XySessionStoreError::validation(format!(
                "v6 migration rejected line {}: {error}",
                line_number + 1
            ))
        })?;
        entries.push(entry);
    }
    Ok(entries)
}
