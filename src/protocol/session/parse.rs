//! Session JSONL parse and version enforcement.

use super::entries::{SESSION_VERSION, SessionEntry};
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

fn unsupported_session_version_msg(v: u32) -> String {
    format!(
        "session header version {v} is not supported (require {SESSION_VERSION}); refusing legacy migrate"
    )
}

/// First JSONL object's session-header version (listing fast-path; no full parse).
pub fn peek_session_header_version(content: &str) -> Option<u32> {
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        return match serde_json::from_str::<SessionEntry>(line) {
            Ok(SessionEntry::Header(h)) => Some(h.version),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::protocol::session::entries::{EntryBase, MessageEntry, SessionEntry, SessionHeader};
    use proptest::prelude::*;

    /// Arbitrary JSON values, including unicode strings and nested
    /// containers — exercises serde escapes and the JSONL line format.
    ///
    /// No bare-f64 leaf: serde_json without its `float_roundtrip` feature
    /// parses floats approximately, so arbitrary bit-pattern floats do NOT
    /// survive a serialize→parse cycle exactly. In-session values are always
    /// already serde_json-normalized (provider output parses through the same
    /// lib), so this models reality rather than hiding a bug.
    fn json_value() -> impl Strategy<Value = serde_json::Value> {
        let leaf = prop_oneof![
            Just(serde_json::Value::Null),
            any::<bool>().prop_map(serde_json::Value::Bool),
            any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
            any::<u64>().prop_map(|n| serde_json::Value::Number(n.into())),
            any::<String>().prop_map(serde_json::Value::String),
        ];
        leaf.prop_recursive(3, 64, 8, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..6).prop_map(serde_json::Value::Array),
                prop::collection::hash_map(".{0,12}", inner, 0..6)
                    .prop_map(|m| { serde_json::Value::Object(m.into_iter().collect()) }),
            ]
        })
    }

    fn message_entry(value: serde_json::Value) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            // entry_type is #[serde(skip)]: the enum tag owns it on disk and
            // parse normalizes it back to empty — the roundtrip pins that.
            base: EntryBase {
                entry_type: String::new(),
                id: "m1".into(),
                parent_id: None,
                timestamp: 7,
            },
            message: value,
        })
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(256))]

        /// Single-line roundtrip: arbitrary message payloads survive a
        /// serialize → parse cycle at the Value level.
        #[test]
        fn message_entry_jsonl_roundtrip(value in json_value()) {
            let entry = message_entry(value);
            let line = serde_json::to_string(&entry).expect("serialize");
            let (entries, warns) = parse_session_jsonl_lines(&line);
            prop_assert_eq!(warns, 0);
            prop_assert_eq!(entries.len(), 1);
            prop_assert_eq!(entries.into_iter().next().unwrap(), entry);
        }

        /// Whole-file roundtrip with header: N arbitrary entries + valid
        /// header parse back identically and pass version enforcement.
        #[test]
        fn session_file_roundtrip(payloads in prop::collection::vec((json_value(), any::<u64>()), 1..8)) {
            let header = SessionEntry::Header(SessionHeader {
                entry_type: String::new(),
                version: SESSION_VERSION,
                id: "s-1".into(),
                timestamp: 0,
                cwd: "/tmp".into(),
                parent_session: None,
            });
            let mut all = vec![header];
            all.extend(payloads.into_iter().map(|(value, ts)| {
                SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: String::new(),
                        id: format!("m{ts}"),
                        parent_id: None,
                        timestamp: ts,
                    },
                    message: value,
                })
            }));
            let content = all
                .iter()
                .map(|e| serde_json::to_string(e).expect("serialize"))
                .collect::<Vec<_>>()
                .join("\n");
            let parsed = parse_session_jsonl(&content).expect("parse with current header");
            prop_assert_eq!(parsed, all);
        }
    }
}
