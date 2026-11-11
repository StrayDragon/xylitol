#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::LogSessionStore;
use super::append::{list_log_files, read_index};
use super::layout::LOG_FORMAT;
use super::project::load_all;
use crate::types::SessionEntry;
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::is_user_message;

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PageToken {
    pub(crate) v: String,
    pub(crate) leaf: String,
    pub(crate) anchor: String,
    pub(crate) dir: String,
    pub(crate) unit: String,
    pub(crate) limit: usize,
}

impl PageToken {
    pub(crate) fn new(
        leaf: impl Into<String>,
        anchor: impl Into<String>,
        dir: impl Into<String>,
        unit: impl Into<String>,
        limit: usize,
    ) -> Self {
        Self {
            v: LOG_FORMAT.into(),
            leaf: leaf.into(),
            anchor: anchor.into(),
            dir: dir.into(),
            unit: unit.into(),
            limit,
        }
    }

    pub(crate) fn encode(&self) -> String {
        let raw = serde_json::to_vec(self).unwrap_or_default();
        base64url_encode(&raw)
    }

    pub(crate) fn decode(raw: &str) -> Option<Self> {
        let bytes = base64url_decode(raw)?;
        let token: Self = serde_json::from_slice(&bytes).ok()?;
        if token.v != LOG_FORMAT {
            return None;
        }
        Some(token)
    }
}

fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::new();
    let mut index = 0;
    while index < input.len() {
        let b0 = input[index] as u32;
        let b1 = if index + 1 < input.len() {
            input[index + 1] as u32
        } else {
            0
        };
        let b2 = if index + 2 < input.len() {
            input[index + 2] as u32
        } else {
            0
        };
        let value = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        if index + 1 < input.len() {
            out.push(ALPHABET[((value >> 6) & 63) as usize] as char);
        }
        if index + 2 < input.len() {
            out.push(ALPHABET[(value & 63) as usize] as char);
        }
        index += 3;
    }
    out
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut values = Vec::with_capacity(input.len());
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        };
        values.push(value as u32);
    }
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    let mut index = 0;
    while index < values.len() {
        let b0 = values[index];
        let b1 = *values.get(index + 1).unwrap_or(&0);
        let b2 = *values.get(index + 2).unwrap_or(&0);
        let b3 = *values.get(index + 3).unwrap_or(&0);
        let value = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        out.push((value >> 16) as u8);
        if index + 2 < values.len() {
            out.push((value >> 8) as u8);
        }
        if index + 3 < values.len() {
            out.push(value as u8);
        }
        index += 4;
    }
    Some(out)
}

fn turn_ranges(entries: &[SessionEntry]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start: Option<usize> = None;
    for (index, entry) in entries.iter().enumerate() {
        if is_user_message(entry) {
            if let Some(previous) = start {
                ranges.push((previous, index));
            }
            start = Some(index);
        }
    }
    if let Some(previous) = start {
        ranges.push((previous, entries.len()));
    }
    ranges
}

pub(crate) async fn load_last_turns(
    store: &LogSessionStore,
    session_id: &str,
    limit: usize,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let entries = load_all(store, session_id).await?;
    let ranges = turn_ranges(&entries);
    let Some((first, _)) = ranges.first() else {
        return Ok(Vec::new());
    };
    let _ = first;
    let start = ranges.len().saturating_sub(limit.max(1));
    let from = ranges.get(start).map(|(start, _)| *start).unwrap_or(0);
    Ok(entries[from..].to_vec())
}

pub(crate) async fn load_page(
    store: &LogSessionStore,
    session_id: &str,
    token: &str,
) -> Result<Vec<SessionEntry>, XySessionStoreError> {
    let Some(token) = PageToken::decode(token) else {
        return load_last_turns(store, session_id, 3).await;
    };
    let entries = load_all(store, session_id).await?;
    let Some(anchor_pos) = entries
        .iter()
        .position(|entry| entry.entry_id() == Some(token.anchor.as_str()))
    else {
        return load_last_turns(store, session_id, token.limit).await;
    };
    let limit = token.limit.max(1);
    match token.unit.as_str() {
        "turn" => {
            let ranges = turn_ranges(&entries);
            let Some(anchor_turn) = ranges
                .iter()
                .position(|(start, end)| anchor_pos >= *start && anchor_pos < *end)
            else {
                return load_last_turns(store, session_id, limit).await;
            };
            if token.dir == "older" {
                let from_turn = anchor_turn.saturating_sub(limit);
                Ok(entries[ranges[from_turn].0..ranges[anchor_turn].0].to_vec())
            } else {
                let to_turn = (anchor_turn + limit).min(ranges.len());
                let to = ranges
                    .get(to_turn)
                    .map(|(start, _)| *start)
                    .unwrap_or(entries.len());
                Ok(entries[ranges[anchor_turn].0..to].to_vec())
            }
        }
        "entry" => {
            if token.dir == "older" {
                let from = anchor_pos.saturating_sub(limit);
                Ok(entries[from..anchor_pos].to_vec())
            } else {
                let to = (anchor_pos + limit).min(entries.len());
                Ok(entries[anchor_pos..to].to_vec())
            }
        }
        _ => load_last_turns(store, session_id, limit).await,
    }
}

pub(crate) async fn read_raw_offsets(
    store: &LogSessionStore,
    session_id: &str,
    file: u32,
) -> Result<Vec<(u64, String)>, XySessionStoreError> {
    let path = store.layout.log_file(session_id, file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(XySessionStoreError::io("read session log", error)),
    };
    let mut out = Vec::new();
    let mut offset = 0u64;
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            offset += 1;
            continue;
        }
        let text = String::from_utf8_lossy(line).into_owned();
        out.push((offset, text));
        offset += line.len() as u64 + 1;
    }
    Ok(out)
}

fn line_has_id(line: &str, entry_id: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    value.get("id").and_then(|value| value.as_str()) == Some(entry_id)
}

pub(crate) async fn locate_entry(
    store: &LogSessionStore,
    session_id: &str,
    entry_id: &str,
) -> Result<Option<(u32, u64)>, XySessionStoreError> {
    let files = list_log_files(store, session_id).await?;
    for file in files.iter().rev() {
        let lines = read_raw_offsets(store, session_id, *file).await?;
        let anchors = read_index(store, session_id, *file).await?;
        let start = anchors.last().map(|anchor| anchor.offset).unwrap_or(0);
        for (offset, line) in lines.iter().filter(|(offset, _)| *offset >= start) {
            if line_has_id(line, entry_id) {
                return Ok(Some((*file, *offset)));
            }
        }
        if start > 0 {
            for (offset, line) in lines.iter().filter(|(offset, _)| *offset < start) {
                if line_has_id(line, entry_id) {
                    return Ok(Some((*file, *offset)));
                }
            }
        }
    }
    Ok(None)
}

pub(crate) async fn load_entries_page(
    store: &LogSessionStore,
    session_id: &str,
    limit: Option<usize>,
    before_entry_id: Option<&str>,
) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
    let entries = load_all(store, session_id).await?;
    Ok(xylitol_plugin_sdk::session::paginate_entries(
        entries,
        limit,
        before_entry_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trip() {
        for raw in [
            b"a".as_slice(),
            b"ab".as_slice(),
            b"abc".as_slice(),
            b"hello world".as_slice(),
        ] {
            let encoded = base64url_encode(raw);
            assert_eq!(base64url_decode(&encoded).as_deref(), Some(raw));
        }
    }

    #[test]
    fn page_token_round_trip() {
        let token = PageToken::new("e_11", "e_06", "older", "turn", 3);
        let encoded = token.encode();
        assert_eq!(PageToken::decode(&encoded), Some(token));
        assert!(PageToken::decode("not-base64").is_none());
    }

    #[test]
    fn stale_version_token_is_rejected() {
        let raw = serde_json::to_vec(&PageToken {
            v: "log-v0".into(),
            leaf: "e".into(),
            anchor: "e".into(),
            dir: "older".into(),
            unit: "turn".into(),
            limit: 3,
        })
        .expect("serialize");
        assert!(PageToken::decode(&base64url_encode(&raw)).is_none());
    }

    #[test]
    fn turn_ranges_start_at_user_messages() {
        use crate::types::{EntryBase, MessageEntry, SessionEntry};
        let msg = |id: &str, role: &str| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: xylitol_plugin_sdk::session::fixture_message_json(role, id),
            })
        };
        let entries = vec![msg("u1", "user"), msg("a1", "assistant"), msg("u2", "user")];
        assert_eq!(turn_ranges(&entries), vec![(0, 2), (2, 3)]);
    }
}
