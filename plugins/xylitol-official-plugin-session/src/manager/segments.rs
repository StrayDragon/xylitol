use std::path::{Component, Path, PathBuf};

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::SessionManager;
use crate::types::SessionEntry;
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::{
    SESSION_VERSION, SealedIndex, SessionManifest, SessionSegment, enforce_session_version,
    parse_session_jsonl_lines,
};

const INITIAL_GENERATION: u64 = 0;

struct AnchorLocation {
    seg_idx: usize,
    pos: usize,
    entries: Vec<SessionEntry>,
}

fn has_earlier_segments(segments: &[&SessionSegment], seg_idx: usize) -> bool {
    segments[..seg_idx]
        .iter()
        .any(|segment| segment.last_entry_id.is_some())
}

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

    pub(super) fn resolve_index_path(
        &self,
        session_id: &str,
        index_path: &str,
    ) -> Result<PathBuf, XySessionStoreError> {
        self.resolve_segment_path(session_id, index_path)
    }

    pub(super) fn sidecar_path_for(segment_path: &str) -> String {
        let Some(stem) = segment_path.strip_suffix(".jsonl") else {
            return format!("{segment_path}.index.json");
        };
        format!("{stem}.index.json")
    }

    pub(super) async fn write_sealed_index_atomically(
        &self,
        session_id: &str,
        index_path: &str,
        index: &SealedIndex,
    ) -> Result<(), XySessionStoreError> {
        let path = self.resolve_index_path(session_id, index_path)?;
        let content = serde_json::to_string_pretty(index)
            .map_err(|e| XySessionStoreError::validation(format!("serialize sealed index: {e}")))?;
        write_file_atomically(&path, &format!("{content}\n")).await
    }

    pub(super) async fn read_sealed_index_opt(
        &self,
        session_id: &str,
        index_path: &str,
    ) -> Option<SealedIndex> {
        let path = match self.resolve_index_path(session_id, index_path) {
            Ok(p) => p,
            Err(_) => return None,
        };
        let raw = tokio::fs::read(&path).await.ok()?;
        match serde_json::from_slice::<SealedIndex>(&raw) {
            Ok(index) => Some(index),
            Err(error) => {
                log::warn!(
                    target: "xylitol::session",
                    "sealed sidecar {index_path} unreadable, falling back to scan: {error}"
                );
                None
            }
        }
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
        let mut index_paths = std::collections::HashSet::new();
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
            if let Some(index_path) = &segment.index_path {
                let resolved = self.resolve_index_path(session_id, index_path)?;
                if paths.contains(&resolved) {
                    return Err(XySessionStoreError::validation(
                        "session manifest index path collides with a segment path",
                    ));
                }
                if !index_paths.insert(resolved) {
                    return Err(XySessionStoreError::validation(
                        "session manifest references a sidecar index more than once",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) async fn read_segment(
        &self,
        session_id: &str,
        segment: &SessionSegment,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        #[cfg(test)]
        self.segment_reads
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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

    pub(super) async fn page_from_segments(
        &self,
        session_id: &str,
        manifest: &SessionManifest,
        limit: Option<usize>,
        before_entry_id: Option<&str>,
    ) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
        let mut segments: Vec<&SessionSegment> = manifest.sealed_segments.iter().collect();
        segments.push(&manifest.active_segment);

        let Some(limit) = limit else {
            let entries = self.load_entries(session_id, manifest).await?;
            return Ok(xylitol_plugin_sdk::session::paginate_entries(
                entries,
                None,
                before_entry_id,
            ));
        };

        let anchor = match before_entry_id {
            Some(anchor) => match self.locate_anchor(session_id, &segments, anchor).await? {
                Some(location) => Some(location),
                None => return Ok((Vec::new(), false)),
            },
            None => None,
        };

        if limit == 0 {
            let has_more = match &anchor {
                Some(location) => {
                    location.pos > 0 || has_earlier_segments(&segments, location.seg_idx)
                }
                None => segments
                    .iter()
                    .any(|segment| segment.last_entry_id.is_some()),
            };
            return Ok((Vec::new(), has_more));
        }

        let mut collected_rev: Vec<SessionEntry> = Vec::new();
        let mut remaining = limit;
        let (mut seg_idx, mut pos, mut cached) = match anchor {
            Some(location) => (location.seg_idx, location.pos, Some(location.entries)),
            None => (segments.len() - 1, usize::MAX, None),
        };
        let mut has_more = false;

        loop {
            let entries = match cached.take() {
                Some(entries) => entries,
                None => self.read_segment(session_id, segments[seg_idx]).await?,
            };
            let take_end = if pos == usize::MAX {
                entries.len()
            } else {
                pos.min(entries.len())
            };
            let take_start = take_end.saturating_sub(remaining);
            let take = take_end - take_start;
            collected_rev.extend(entries[take_start..take_end].iter().rev().cloned());
            remaining -= take;
            if remaining == 0 {
                has_more = take_start > 0 || has_earlier_segments(&segments, seg_idx);
                break;
            }
            if seg_idx == 0 {
                break;
            }
            seg_idx -= 1;
            pos = usize::MAX;
        }

        collected_rev.reverse();
        Ok((collected_rev, has_more))
    }

    async fn locate_anchor(
        &self,
        session_id: &str,
        segments: &[&SessionSegment],
        anchor: &str,
    ) -> Result<Option<AnchorLocation>, XySessionStoreError> {
        for seg_idx in (0..segments.len()).rev() {
            let segment = segments[seg_idx];
            if segment.first_entry_id.as_deref() == Some(anchor) {
                return Ok(Some(AnchorLocation {
                    seg_idx,
                    pos: 0,
                    entries: Vec::new(),
                }));
            }
            if segment.last_entry_id.as_deref() != Some(anchor)
                && let Some(index_path) = &segment.index_path
                && let Some(index) = self.read_sealed_index_opt(session_id, index_path).await
                && !index.contains_entry(anchor)
            {
                continue;
            }
            let entries = self.read_segment(session_id, segment).await?;
            if let Some(pos) = entries
                .iter()
                .position(|entry| entry.entry_id() == Some(anchor))
            {
                return Ok(Some(AnchorLocation {
                    seg_idx,
                    pos,
                    entries,
                }));
            }
        }
        Ok(None)
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
            index_path: None,
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

    pub(super) async fn update_manifest_after_append(
        &self,
        session_id: &str,
        mut manifest: SessionManifest,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        if let Some(entry_id) = entry.entry_id() {
            let entry_id = entry_id.to_owned();
            if manifest.active_segment.first_entry_id.is_none() {
                manifest.active_segment.first_entry_id = Some(entry_id.clone());
            }
            manifest.active_segment.last_entry_id = Some(entry_id.clone());
            manifest.leaf_entry_id = Some(entry_id);
        }
        if matches!(entry, SessionEntry::Header(_)) {
            manifest.active_segment.includes_header = true;
        }
        self.write_manifest_atomically(session_id, &manifest).await
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
            let mut old_paths: Vec<String> = old
                .sealed_segments
                .iter()
                .map(|segment| segment.path.clone())
                .collect();
            old_paths.push(old.active_segment.path.clone());
            old_paths.extend(
                old.sealed_segments
                    .iter()
                    .filter_map(|segment| segment.index_path.clone()),
            );
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
}

pub(super) fn legacy_unsupported_error(session_id: &str) -> XySessionStoreError {
    log::warn!(
        target: "xylitol::session",
        "legacy session {session_id} rejected: no v7 manifest, no automatic migration (zero-compat)"
    );
    XySessionStoreError::unsupported(
        "legacy session storage; export it from an older version and import into a v7 session",
    )
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
