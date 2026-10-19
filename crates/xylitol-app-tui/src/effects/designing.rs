//! Designing preview inject (process-local; not a product slash).
//!
//! c2740/c2930: scenes are seeded into an in-memory store and applied to the
//! transcript directly; no `protocol::Command` / remote unary.
//! **designing preview** = 把命名场景注入同一 TUI 画布的预览设计。

use xylitol_tui::Terminal;

use crate::debug_fixtures::PreviewInject;
use crate::driver::{DebugSceneLoad, XyDriver};
use crate::host::HostSession;
use xylitol_plugin_sdk::ports::XySessionStore;

pub(crate) async fn inject_scene<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    scene: String,
) {
    let scene = scene.trim().to_ascii_lowercase();
    if scene.is_empty() || scene == "list" {
        session.push_scroll_notice(crate::debug_fixtures::list_note());
    } else if let Some(meta) = crate::debug_fixtures::find_scene(&scene) {
        match (meta.inject, meta.id) {
            (
                PreviewInject::FixedZone(crate::debug_fixtures::FixedZoneOp::SlotModels),
                "verify-smoke",
            ) => {
                super::debug_verify::run_verify_smoke(session, driver).await;
            }
            (PreviewInject::FixedZone(op), _) => {
                session.apply_fixed_zone_op(op);
            }
            (PreviewInject::LiveTape, "activity-fold-live") => {
                super::debug_activity_fold::run_activity_fold_live(session);
            }
            (PreviewInject::LiveXy, _) => {
                super::debug_activity_fold::run_preview_live_xy(session);
            }
            (PreviewInject::Resume, _) => match load_and_bind_scene(driver, meta.id).await {
                Ok(load) => {
                    driver.bind_debug_scene(&load);
                    session.apply_debug_scene(load);
                }
                Err(e) => note_load_err(session, "tui.designing_preview", &e, e.to_string()),
            },
            (inject, id) => {
                session.push_scroll_notice(format!(
                    "{id}: inject {inject:?} has no designing preview runner"
                ));
            }
        }
    } else {
        match load_and_bind_scene(driver, &scene).await {
            Ok(load) => {
                driver.bind_debug_scene(&load);
                session.apply_debug_scene(load);
            }
            Err(e) => note_load_err(session, "tui.designing_preview", &e, e.to_string()),
        }
    }
    let _ = session.render_now();
}

/// Seed a scene into the driver's store when the Host is in-process (designing
/// uses an ephemeral store). Otherwise a throwaway in-memory manager — never
/// the caller's current JSONL file, never the RPC surface.
async fn load_and_bind_scene(
    driver: &mut dyn XyDriver,
    scene: &str,
) -> Result<DebugSceneLoad, crate::driver_error::XyDriverError> {
    use crate::driver_error::XyDriverError;

    let short = &uuid::Uuid::new_v4().to_string()[..8];
    let canonical = crate::debug_fixtures::resolve_scene_id(scene).ok_or_else(|| {
        XyDriverError::invalid_input(format!(
            "unknown designing scene: {scene}\n{}",
            crate::debug_fixtures::list_note()
        ))
    })?;
    let session_id = format!("designing-{canonical}-{short}");
    let bound = driver.session_store();
    let ephemeral = EphemeralStore::new();
    let store: &dyn XySessionStore = bound.as_deref().unwrap_or(&ephemeral);
    store
        .create(&session_id, None, None)
        .await
        .map_err(XyDriverError::from)?;
    let canonical = crate::debug_fixtures::seed_scene(store, &session_id, scene).await?;
    let entries = store.load_entries(&session_id).await?;
    if bound.is_some() {
        let _ = crate::dispatch::dispatch(
            driver,
            xylitol_protocol::Command::SwitchSession {
                session_path: session_id.clone(),
            },
        )
        .await;
    }
    Ok(DebugSceneLoad {
        note: format!("designing preview `{canonical}` → local in-memory session {session_id}"),
        session_id,
        entries,
        model: None,
    })
}

fn note_load_err<T: Terminal>(
    session: &mut HostSession<T>,
    target: &str,
    err: &crate::driver_error::XyDriverError,
    text: String,
) {
    log::warn!(target: "xylitol::tui", "{target} failed: {err}");
    session.push_toast_notice(text);
}

/// Throwaway in-memory store for designing when the driver has no session_store
/// (scripted / remote). Never the caller's JSONL file.
struct EphemeralStore {
    sessions: std::sync::Mutex<
        std::collections::HashMap<String, Vec<xylitol_plugin_sdk::session::SessionEntry>>,
    >,
    leaves: std::sync::Mutex<std::collections::HashMap<String, Option<String>>>,
}

impl EphemeralStore {
    fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            leaves: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl XySessionStore for EphemeralStore {
    async fn exists(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .map(|m| m.contains_key(session_id))
            .unwrap_or(false)
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<
        Vec<xylitol_plugin_sdk::session::SessionEntry>,
        xylitol_plugin_sdk::error::XySessionStoreError,
    > {
        self.sessions
            .lock()
            .ok()
            .and_then(|m| m.get(session_id).cloned())
            .ok_or_else(|| xylitol_plugin_sdk::error::XySessionStoreError::not_found(session_id))
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &xylitol_plugin_sdk::session::SessionEntry,
    ) -> Result<(), xylitol_plugin_sdk::error::XySessionStoreError> {
        let new_id = uuid::Uuid::new_v4().to_string();
        let parent_id = self.leaf_id(session_id);
        let stamped = stamp_entry_ids(entry, &new_id, parent_id.as_deref());
        let mut map = self.sessions.lock().map_err(|_| {
            xylitol_plugin_sdk::error::XySessionStoreError::io(
                "lock",
                std::io::Error::other("poison"),
            )
        })?;
        let list = map
            .get_mut(session_id)
            .ok_or_else(|| xylitol_plugin_sdk::error::XySessionStoreError::not_found(session_id))?;
        list.push(stamped);
        drop(map);
        self.set_leaf(session_id, Some(&new_id));
        Ok(())
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<
        xylitol_plugin_sdk::session::SessionContext,
        xylitol_plugin_sdk::error::XySessionStoreError,
    > {
        let _ = self.load_entries(session_id).await?;
        Ok(xylitol_plugin_sdk::session::SessionContext {
            messages: Vec::new(),
            thinking_level: String::new(),
            model: None,
        })
    }

    async fn create(
        &self,
        id: &str,
        _cwd: Option<&str>,
        _parent: Option<&str>,
    ) -> Result<(), xylitol_plugin_sdk::error::XySessionStoreError> {
        self.sessions
            .lock()
            .map_err(|_| {
                xylitol_plugin_sdk::error::XySessionStoreError::io(
                    "lock",
                    std::io::Error::other("poison"),
                )
            })?
            .insert(id.to_string(), Vec::new());
        Ok(())
    }

    async fn fork(
        &self,
        _parent_id: &str,
        _child_id: &str,
        _at_entry_id: &str,
        _position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<(), xylitol_plugin_sdk::error::XySessionError> {
        Err(xylitol_plugin_sdk::error::XySessionError::from(
            xylitol_plugin_sdk::error::XySessionStoreError::unsupported("fork"),
        ))
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        if let Ok(mut m) = self.leaves.lock() {
            m.insert(session_id.to_string(), entry_id.map(str::to_string));
        }
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        self.leaves
            .lock()
            .ok()
            .and_then(|m| m.get(session_id).cloned().flatten())
    }
}

fn stamp_entry_ids(
    entry: &xylitol_plugin_sdk::session::SessionEntry,
    id: &str,
    parent_id: Option<&str>,
) -> xylitol_plugin_sdk::session::SessionEntry {
    use xylitol_plugin_sdk::session::{LabelEntry, MessageEntry, SessionEntry};
    match entry {
        SessionEntry::Message(m) => SessionEntry::Message(MessageEntry {
            base: xylitol_plugin_sdk::session::EntryBase {
                entry_type: m.base.entry_type.clone(),
                id: id.to_string(),
                parent_id: parent_id.map(str::to_string),
                timestamp: m.base.timestamp,
            },
            message: m.message.clone(),
        }),
        SessionEntry::Label(l) => SessionEntry::Label(LabelEntry {
            base: xylitol_plugin_sdk::session::EntryBase {
                entry_type: l.base.entry_type.clone(),
                id: id.to_string(),
                parent_id: parent_id.map(str::to_string),
                timestamp: l.base.timestamp,
            },
            target_id: l.target_id.clone(),
            label: l.label.clone(),
        }),
        other => other.clone(),
    }
}
