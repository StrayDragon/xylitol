use serde::{Deserialize, Serialize};

pub const CUSTOM_TYPE_CONTEXT_INJECTED: &str = "contextInjected";

pub const QUEUE_ITEM_PREVIEW_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueItemKind {
    Steer,
    FollowUp,
    #[default]
    Unknown,
}

impl QueueItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steer => "steer",
            Self::FollowUp => "follow_up",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_wire(raw: &str) -> Self {
        match raw {
            "steer" => Self::Steer,
            "follow_up" => Self::FollowUp,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub kind: QueueItemKind,
    pub preview: String,
}

impl QueueItem {
    pub fn preview_of(text: &str) -> String {
        text.chars().take(QUEUE_ITEM_PREVIEW_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::entries::{ContextInjectedEntry, EntryBase};
    use super::*;

    #[test]
    fn queue_item_preview_truncates_to_eighty_chars() {
        let long = "a".repeat(200);
        assert_eq!(QueueItem::preview_of(&long).chars().count(), 80);
        assert_eq!(QueueItem::preview_of("short"), "short");
    }

    #[test]
    fn unknown_queue_kind_degrades_without_panic() {
        assert_eq!(
            QueueItemKind::from_wire("totally_unknown"),
            QueueItemKind::Unknown
        );
        let item = QueueItem {
            id: "q1".into(),
            kind: QueueItemKind::from_wire("totally_unknown"),
            preview: "x".into(),
        };
        assert_eq!(item.kind, QueueItemKind::Unknown);
        let steer: QueueItem =
            serde_json::from_str(r#"{"id":"q2","kind":"steer","preview":"y"}"#).expect("steer");
        assert_eq!(steer.kind, QueueItemKind::Steer);
        let follow: QueueItem =
            serde_json::from_str(r#"{"id":"q3","kind":"follow_up","preview":"z"}"#)
                .expect("follow");
        assert_eq!(follow.kind, QueueItemKind::FollowUp);
    }

    #[test]
    fn queue_item_roundtrips_on_wire() {
        let item = QueueItem {
            id: "q1".into(),
            kind: QueueItemKind::FollowUp,
            preview: "hi".into(),
        };
        let text = serde_json::to_string(&item).expect("encode");
        let back: QueueItem = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, item);
        assert!(text.contains("\"follow_up\""));
    }

    #[test]
    fn context_injected_entry_serializes_camel_case_payload() {
        let entry = ContextInjectedEntry {
            base: EntryBase {
                entry_type: "contextInjected".into(),
                id: "e1".into(),
                parent_id: None,
                timestamp: 1,
            },
            injector_id: "skills".into(),
            point: "turnSubmission".into(),
            fingerprint: Some("fp".into()),
            preview: Some("pre".into()),
        };
        let session_entry = super::super::entries::SessionEntry::ContextInjected(entry);
        let line = serde_json::to_string(&session_entry).expect("encode session entry");
        let back: super::super::entries::SessionEntry =
            serde_json::from_str(&line).expect("decode session entry");
        assert!(matches!(
            back,
            super::super::entries::SessionEntry::ContextInjected(_)
        ));
        assert_eq!(back.entry_type(), "contextInjected");
        assert!(line.contains("\"type\":\"contextInjected\""), "{line}");
        assert!(line.contains("\"injectorId\":\"skills\""), "{line}");
        assert!(line.contains("\"point\":\"turnSubmission\""), "{line}");
        assert!(line.contains("\"fingerprint\":\"fp\""), "{line}");
    }
}
