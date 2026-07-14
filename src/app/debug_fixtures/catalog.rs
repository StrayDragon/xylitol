//! Scene catalog: stable ids + human descriptions (for `/debug` list + completion).

/// One hand-test fixture exposed as `/debug <id>`.
#[derive(Debug, Clone, Copy)]
pub struct DebugSceneMeta {
    pub id: &'static str,
    pub description: &'static str,
}

/// All scenes. Prefer descriptive ids so `/debug <Tab>` is self-explanatory.
pub const DEBUG_SCENES: &[DebugSceneMeta] = &[
    DebugSceneMeta {
        id: "session-tree-multiturn",
        description: "Multi-turn MessageHistory for Search/Help/fold hand-test",
    },
    DebugSceneMeta {
        id: "session-tree-labeled",
        description: "Tree with [bookmark] annotation for labeled-only / Shift+L",
    },
];

/// `(id, description)` for [`xylitol_tui::SlashArgCompletionSource`].
pub fn completion_catalog() -> Vec<(String, String)> {
    DEBUG_SCENES
        .iter()
        .map(|s| (s.id.to_string(), s.description.to_string()))
        .collect()
}

/// System-note body for bare `/debug` (list).
pub fn list_note() -> String {
    let mut lines = vec!["debug scenes (try /debug <id>):".to_string()];
    for s in DEBUG_SCENES {
        lines.push(format!("  {} — {}", s.id, s.description));
    }
    lines.join("\n")
}

/// Accept canonical id or short legacy aliases from early c710 drafts.
pub fn resolve_scene_id(raw: &str) -> Option<&'static str> {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() || key == "list" {
        return None;
    }
    for s in DEBUG_SCENES {
        if s.id == key {
            return Some(s.id);
        }
    }
    // Short aliases so old muscle memory still works once.
    match key.as_str() {
        "tree-branch" | "multiturn" => Some("session-tree-multiturn"),
        "tree-labeled" | "labeled" => Some("session-tree-labeled"),
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
        assert_eq!(resolve_scene_id("list"), None);
        assert_eq!(resolve_scene_id("nope"), None);
    }
}
