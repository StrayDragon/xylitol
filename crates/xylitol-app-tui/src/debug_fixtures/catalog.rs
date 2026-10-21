#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewInject {
    LiveXy,
    LiveTape,
    Resume,
    FixedZone(FixedZoneOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedZoneOp {
    Toast,
    NextTurnCue,
    SlotModels,
    SlotChoice,
    SlotTree,
}

impl PreviewInject {
    pub const fn seeds_jsonl(self) -> bool {
        matches!(self, Self::Resume)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DebugSceneMeta {
    pub id: &'static str,
    pub description: &'static str,
    pub inject: PreviewInject,
}

pub const DEBUG_SCENES: &[DebugSceneMeta] = &[
    DebugSceneMeta {
        id: "session-tree-multiturn",
        description: "Multi-turn MessageHistory for Search/Help/fold hand-test",
        inject: PreviewInject::Resume,
    },
    DebugSceneMeta {
        id: "session-tree-labeled",
        description: "Tree with [bookmark] annotation for labeled-only / Shift+L",
        inject: PreviewInject::Resume,
    },
    DebugSceneMeta {
        id: "session-tree-branched",
        description: "Sibling branches under one parent (fold / ←→ jump hand-test)",
        inject: PreviewInject::Resume,
    },
    DebugSceneMeta {
        id: "ao-perf-scroll",
        description: "Long transcript (~80 turns) for AO wheel/select CPU hand-test",
        inject: PreviewInject::Resume,
    },
    DebugSceneMeta {
        id: "verify-smoke",
        description: "UI-only B4/B7 smoke (no LLM, no /exit); report via scroll notice",
        inject: PreviewInject::FixedZone(FixedZoneOp::SlotModels),
    },
    DebugSceneMeta {
        id: "activity-fold-live",
        description: "Live window tape + Ask Choice (no LLM); answer to close the turn",
        inject: PreviewInject::LiveTape,
    },
    DebugSceneMeta {
        id: "activity-fold-live-xy",
        description: "LiveXy: Thinking + read via HostSession::step (no JSONL, no Choice tape)",
        inject: PreviewInject::LiveXy,
    },
    DebugSceneMeta {
        id: "activity-fold-resume",
        description: "Ended session with tools + answered Ask; persist and /resume fold",
        inject: PreviewInject::Resume,
    },
];

pub fn scene_catalog() -> Vec<(String, String)> {
    DEBUG_SCENES
        .iter()
        .map(|s| (s.id.to_string(), s.description.to_string()))
        .collect()
}

pub fn list_note() -> String {
    let mut lines = vec!["designing scenes:".to_string()];
    for s in DEBUG_SCENES {
        lines.push(format!("  {} — {}", s.id, s.description));
    }
    lines.join("\n")
}

pub fn find_scene(raw: &str) -> Option<&'static DebugSceneMeta> {
    let key = raw.trim().to_ascii_lowercase();
    DEBUG_SCENES.iter().find(|s| s.id == key)
}

pub fn resolve_scene_id(raw: &str) -> Option<&'static str> {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() || key == "list" {
        return None;
    }
    if let Some(s) = find_scene(&key) {
        return s.inject.seeds_jsonl().then_some(s.id);
    }
    match key.as_str() {
        "tree-branch" | "multiturn" => Some("session-tree-multiturn"),
        "tree-labeled" | "labeled" => Some("session-tree-labeled"),
        "branched" | "tree-branched" => Some("session-tree-branched"),
        "long-transcript" | "ao-long-transcript" | "perf-scroll" => Some("ao-perf-scroll"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_canonical_and_alias() {
        assert_eq!(
            resolve_scene_id("session-tree-multiturn"),
            Some("session-tree-multiturn")
        );
        assert_eq!(
            resolve_scene_id("tree-branch"),
            Some("session-tree-multiturn")
        );
        assert_eq!(resolve_scene_id("branched"), Some("session-tree-branched"));
        assert_eq!(resolve_scene_id("long-transcript"), Some("ao-perf-scroll"));
        assert_eq!(resolve_scene_id("ao-perf-scroll"), Some("ao-perf-scroll"));
        assert_eq!(resolve_scene_id("verify-smoke"), None);
        assert_eq!(resolve_scene_id("activity-fold-live"), None);
        assert_eq!(resolve_scene_id("activity-fold-live-xy"), None);
        assert_eq!(
            resolve_scene_id("activity-fold-resume"),
            Some("activity-fold-resume")
        );
        assert_eq!(resolve_scene_id("list"), None);
        assert_eq!(resolve_scene_id("nope"), None);
    }

    #[test]
    fn every_scene_declares_inject_seam() {
        use PreviewInject::*;
        let by_id: std::collections::HashMap<&str, PreviewInject> =
            DEBUG_SCENES.iter().map(|s| (s.id, s.inject)).collect();
        assert_eq!(by_id["activity-fold-live"], LiveTape);
        assert_eq!(by_id["activity-fold-live-xy"], LiveXy);
        assert_eq!(by_id["activity-fold-resume"], Resume);
        assert_eq!(by_id["verify-smoke"], FixedZone(FixedZoneOp::SlotModels));
        let _seams = [
            LiveXy,
            LiveTape,
            Resume,
            FixedZone(FixedZoneOp::Toast),
            FixedZone(FixedZoneOp::NextTurnCue),
            FixedZone(FixedZoneOp::SlotModels),
            FixedZone(FixedZoneOp::SlotChoice),
            FixedZone(FixedZoneOp::SlotTree),
        ];
        assert_eq!(_seams.len(), 8);
        assert_eq!(by_id["session-tree-multiturn"], Resume);
        assert_eq!(by_id["ao-perf-scroll"], Resume);
        assert!(
            DEBUG_SCENES.iter().all(|s| !s.id.is_empty()),
            "catalog ids must stay non-empty"
        );
    }
}
