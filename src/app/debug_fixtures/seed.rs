//! Persist fixture entries into an already-created session.

use crate::domain::session_types::{
    EntryBase, LabelEntry, MessageEntry, SessionEntry, fixture_message_json,
};
use crate::runtime_protocol::XySessionStore;

use super::catalog::resolve_scene_id;

fn empty_base(entry_type: &str) -> EntryBase {
    EntryBase {
        entry_type: entry_type.into(),
        id: String::new(),
        parent_id: None,
        timestamp: String::new(),
    }
}

fn user_msg(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: empty_base("message"),
        message: fixture_message_json("user", text),
    })
}

fn assistant_msg(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: empty_base("message"),
        message: fixture_message_json("assistant", text),
    })
}

/// Seed `session_id` for a known scene id (canonical or alias).
pub async fn seed_scene(
    store: &dyn XySessionStore,
    session_id: &str,
    scene: &str,
) -> Result<&'static str, String> {
    let id = resolve_scene_id(scene).ok_or_else(|| {
        format!(
            "unknown debug scene: {scene}\n{}",
            super::catalog::list_note()
        )
    })?;
    match id {
        "session-tree-multiturn" => seed_multiturn(store, session_id).await?,
        "session-tree-labeled" => seed_labeled(store, session_id).await?,
        "session-tree-branched" => seed_branched(store, session_id).await?,
        _ => return Err(format!("unhandled debug scene id: {id}")),
    }
    Ok(id)
}

async fn seed_multiturn(store: &dyn XySessionStore, session_id: &str) -> Result<(), String> {
    store
        .append_session_entry(session_id, &user_msg("debug: hi"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: hello from fixture"))
        .await?;
    store
        .append_session_entry(session_id, &user_msg("debug: 你好"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: 有什么可以帮你？"))
        .await?;
    Ok(())
}

/// Same-session sibling branches: rewind leaf after the first spine, then grow an alt path.
///
/// ```text
/// u_root → a_root ─┬─ u_main → a_main
///                  └─ u_alt  → a_alt
/// ```
async fn seed_branched(store: &dyn XySessionStore, session_id: &str) -> Result<(), String> {
    store
        .append_session_entry(session_id, &user_msg("debug: root"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: root reply"))
        .await?;
    let fork_parent = store
        .leaf_id(session_id)
        .ok_or_else(|| "debug session-tree-branched: missing fork parent leaf".to_string())?;

    store
        .append_session_entry(session_id, &user_msg("debug: main branch"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: main leaf"))
        .await?;

    store.set_leaf(session_id, Some(&fork_parent));
    store
        .append_session_entry(session_id, &user_msg("debug: alt branch"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: alt leaf"))
        .await?;
    Ok(())
}

async fn seed_labeled(store: &dyn XySessionStore, session_id: &str) -> Result<(), String> {
    store
        .append_session_entry(session_id, &user_msg("debug: labeled root"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: labeled reply"))
        .await?;
    let entries = store.load_entries(session_id).await?;
    let target = entries
        .iter()
        .find_map(|e| match e {
            SessionEntry::Message(m)
                if m.message
                    .get("role")
                    .and_then(|r| r.as_str())
                    .is_some_and(|r| r == "user") =>
            {
                e.entry_id().map(str::to_string)
            }
            _ => None,
        })
        .ok_or_else(|| "debug session-tree-labeled: missing user entry".to_string())?;
    store
        .append_session_entry(
            session_id,
            &SessionEntry::Label(LabelEntry {
                base: empty_base("label"),
                target_id: target,
                label: Some("bookmark".into()),
            }),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session_types::message_text;
    use crate::infra::session::SessionManager;

    #[tokio::test]
    async fn seed_multiturn_has_user_text() {
        let mgr = SessionManager::in_memory();
        mgr.create("d1", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d1", "session-tree-multiturn")
            .await
            .unwrap();
        assert_eq!(id, "session-tree-multiturn");
        let entries = mgr.load_entries("d1").await.unwrap();
        let texts: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => {
                    let t = message_text(&m.message);
                    (!t.is_empty()).then_some(t)
                }
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("debug: hi")), "{texts:?}");
    }

    #[tokio::test]
    async fn seed_labeled_alias_has_label() {
        let mgr = SessionManager::in_memory();
        mgr.create("d2", Some("."), None).await.unwrap();
        seed_scene(&mgr, "d2", "tree-labeled").await.unwrap();
        let entries = mgr.load_entries("d2").await.unwrap();
        assert!(
            entries.iter().any(|e| matches!(
                e,
                SessionEntry::Label(LabelEntry {
                    label: Some(l),
                    ..
                }) if l == "bookmark"
            )),
            "{entries:?}"
        );
    }

    #[tokio::test]
    async fn seed_branched_has_sibling_children() {
        use crate::domain::session_types::build_session_tree;

        let mgr = SessionManager::in_memory();
        mgr.create("d3", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d3", "branched").await.unwrap();
        assert_eq!(id, "session-tree-branched");
        let tree = build_session_tree(&mgr.load_entries("d3").await.unwrap());
        let max_siblings = tree
            .iter()
            .flat_map(|n| n.children.iter())
            .map(|n| n.children.len())
            .max()
            .unwrap_or(0);
        assert!(
            max_siblings >= 2,
            "expected a parent with ≥2 children, got max={max_siblings}; tree={tree:#?}"
        );
    }
}
