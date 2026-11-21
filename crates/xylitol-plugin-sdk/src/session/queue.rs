use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::EntryBase;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextInjectedEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub injector_id: String,
    pub point: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

impl ContextInjectedEntry {
    pub fn to_custom_entry(&self) -> Value {
        serde_json::json!({
            "injectorId": self.injector_id,
            "point": self.point,
            "fingerprint": self.fingerprint,
            "preview": self.preview,
        })
    }
}

#[cfg(test)]
mod tests {
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
        let value = entry.to_custom_entry();
        assert_eq!(value["injectorId"], "skills");
        assert_eq!(value["point"], "turnSubmission");
        assert_eq!(value["fingerprint"], "fp");
    }
}
