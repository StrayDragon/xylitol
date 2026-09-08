//! Current xylitol session identity for Langfuse OTEL attributes (c1480).
//!
//! Lives in ai-bridge so both `llm.request` and agent ReAct obs can read
//! it without agent → infra edges. App driver updates on session switch/name.
//!
//! Resolution (Keybindings-style) for **idle** paths (no generate options):
//! 1. thread-local [`ObsSessionScope`] if entered (tests / sync inject)
//! 2. else process `Mutex` slot (production — survives tokio worker hops)
//!
//! Overlapping generate (c2590): the snapshot on
//! [`crate::thinking::AiBridgeGenerateOptions::obs_session`] is the authority.
//! HTTP hooks / `llm.request` MUST use that copy and MUST NOT re-read this slot
//! mid-request. The process slot remains a fallback for paths that never received
//! options (remote count, tests). Scope is for cargo-test isolation.

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

pub use crate::thinking::ObsSessionContext;

fn slot() -> &'static Mutex<ObsSessionContext> {
    static SLOT: OnceLock<Mutex<ObsSessionContext>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(ObsSessionContext::default()))
}

thread_local! {
    /// Tests / sync inject: prefer this over the process slot while the guard lives.
    static SCOPED: RefCell<Option<ObsSessionContext>> = const { RefCell::new(None) };
}

/// RAII install of a thread-local obs session context.
///
/// Restores the previous scoped value on drop (nested scopes supported).
/// Prefer for unit tests; do **not** wrap production HTTP paths (worker hops).
pub struct ObsSessionScope {
    prev: Option<ObsSessionContext>,
}

impl ObsSessionScope {
    pub fn enter(ctx: ObsSessionContext) -> Self {
        let prev = SCOPED.with(|slot| slot.borrow_mut().replace(ctx));
        Self { prev }
    }
}

impl Drop for ObsSessionScope {
    fn drop(&mut self) {
        SCOPED.with(|slot| {
            *slot.borrow_mut() = self.prev.take();
        });
    }
}

fn with_active_mut<R>(f: impl FnOnce(&mut ObsSessionContext) -> R) -> R {
    if SCOPED.with(|slot| slot.borrow().is_some()) {
        return SCOPED.with(|slot| {
            let mut g = slot.borrow_mut();
            f(g.as_mut().expect("scoped obs session"))
        });
    }
    let mut g = slot().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

/// Replace session id; clears name unless `name` is `Some`.
pub fn set_obs_session(session_id: impl Into<String>, name: Option<String>) {
    let id = session_id.into();
    let id = id.trim();
    with_active_mut(|g| {
        if id.is_empty() {
            *g = ObsSessionContext::default();
            return;
        }
        g.session_id = Some(id.to_string());
        g.session_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        g.parent_session_id = None;
        g.fork_at_entry_id = None;
        g.llm_gateway_session_id = None;
    });
}

/// Update display name only (keeps session id). Empty / whitespace clears name.
pub fn set_obs_session_name(name: Option<&str>) {
    with_active_mut(|g| {
        g.session_name = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string);
    });
}

pub fn clear_obs_session() {
    with_active_mut(|g| *g = ObsSessionContext::default());
}

pub fn obs_session_context() -> ObsSessionContext {
    if let Some(ctx) = SCOPED.with(|slot| slot.borrow().clone()) {
        return ctx;
    }
    slot().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Fastrace property pairs for Langfuse session mapping from an explicit snapshot.
pub fn langfuse_session_properties_from(ctx: &ObsSessionContext) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(6);
    if let Some(id) = ctx.session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("langfuse.session.id".into(), id.clone()));
        out.push(("xylitol.session.id".into(), id));
    }
    if let Some(gw) = ctx.llm_gateway_session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.llm_gateway_session_id".into(), gw));
    }
    if let Some(parent) = ctx.parent_session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.parent_session_id".into(), parent));
    }
    if let Some(cut) = ctx.fork_at_entry_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.fork_at_entry_id".into(), cut));
    }
    if let Some(name) = ctx.session_name.clone().filter(|s| !s.is_empty()) {
        out.push(("langfuse.trace.metadata.session_name".into(), name));
    }
    out
}

/// Idle-path session properties (process slot / TLS). Generate MUST use
/// [`langfuse_session_properties_from`] with the options snapshot instead.
pub fn langfuse_session_properties() -> Vec<(String, String)> {
    langfuse_session_properties_from(&obs_session_context())
}

/// Append Langfuse observation type (+ session + llm obs lane) from a snapshot.
pub fn langfuse_observation_properties_from(
    observation_type: &str,
    ctx: &ObsSessionContext,
) -> Vec<(String, String)> {
    let mut out = vec![(
        "langfuse.observation.type".into(),
        observation_type.to_string(),
    )];
    out.extend(langfuse_session_properties_from(ctx));
    out.extend(xylitol_obs_lane_properties(XYLITOL_OBS_LANE_LLM));
    out
}

/// Idle-path observation properties (process slot / TLS).
pub fn langfuse_observation_properties(observation_type: &str) -> Vec<(String, String)> {
    langfuse_observation_properties_from(observation_type, &obs_session_context())
}

/// Fastrace attribute for Collector / consumer routing (`llm` | `infra`).
pub const XYLITOL_OBS_LANE_ATTR: &str = "xylitol.obs.lane";
/// LLM / agent product process-tree lane.
pub const XYLITOL_OBS_LANE_LLM: &str = "llm";

/// Property pairs for `xylitol.obs.lane`.
pub fn xylitol_obs_lane_properties(lane: &str) -> Vec<(String, String)> {
    vec![(XYLITOL_OBS_LANE_ATTR.into(), lane.to_string())]
}

/// Generation helpers from an explicit session snapshot (overlapping generate).
///
/// Prefer `langfuse.observation.model.name` only (Langfuse OTEL mapping; `langfuse.*`
/// takes precedence). Do not also set bare `model` / `gen_ai.request.model` — same
/// mapped field, and bare `model` can force generation typing on unrelated spans.
pub fn langfuse_generation_properties_from(
    model: &str,
    ctx: &ObsSessionContext,
) -> Vec<(String, String)> {
    let mut out = langfuse_observation_properties_from("generation", ctx);
    if !model.is_empty() {
        out.push(("langfuse.observation.model.name".into(), model.to_string()));
    }
    out
}

/// Idle-path generation helpers (process slot / TLS).
pub fn langfuse_generation_properties(model: &str) -> Vec<(String, String)> {
    langfuse_generation_properties_from(model, &obs_session_context())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_without_name() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-1", None);
        let p = langfuse_session_properties();
        assert_eq!(
            p,
            vec![
                ("langfuse.session.id".into(), "sid-1".into()),
                ("xylitol.session.id".into(), "sid-1".into()),
            ]
        );
        assert!(!p.iter().any(|(k, _)| k.contains("session_name")));
    }

    #[test]
    fn id_with_name_and_rename() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-2", Some("alpha".into()));
        let p = langfuse_session_properties();
        assert!(p.contains(&("langfuse.session.id".into(), "sid-2".into())));
        assert!(p.contains(&(
            "langfuse.trace.metadata.session_name".into(),
            "alpha".into()
        )));
        set_obs_session_name(Some("beta"));
        let p2 = langfuse_session_properties();
        assert!(p2.contains(&("langfuse.session.id".into(), "sid-2".into())));
        assert!(p2.contains(&("langfuse.trace.metadata.session_name".into(), "beta".into())));
        set_obs_session_name(Some("  "));
        let p3 = langfuse_session_properties();
        assert_eq!(
            p3,
            vec![
                ("langfuse.session.id".into(), "sid-2".into()),
                ("xylitol.session.id".into(), "sid-2".into()),
            ]
        );
    }

    #[test]
    fn generation_properties_include_type_and_model() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-g", None);
        let p = langfuse_generation_properties("gpt-test");
        assert!(p.contains(&("langfuse.observation.type".into(), "generation".into())));
        assert!(p.contains(&("langfuse.observation.model.name".into(), "gpt-test".into())));
        assert!(
            !p.iter()
                .any(|(k, _)| k == "gen_ai.request.model" || k == "model")
        );
        assert!(p.contains(&("xylitol.session.id".into(), "sid-g".into())));
        assert!(p.contains(&(XYLITOL_OBS_LANE_ATTR.into(), XYLITOL_OBS_LANE_LLM.into())));
    }

    #[test]
    fn scope_does_not_pollute_process_slot() {
        clear_obs_session();
        {
            let _g = ObsSessionScope::enter(ObsSessionContext::default());
            set_obs_session("scoped-only", None);
            assert_eq!(
                obs_session_context().session_id.as_deref(),
                Some("scoped-only")
            );
        }
        assert!(obs_session_context().session_id.is_none());
    }

    #[test]
    fn properties_from_snapshot_ignore_process_slot() {
        let _g = ObsSessionScope::enter(ObsSessionContext {
            session_id: Some("process-wrong".into()),
            session_name: Some("wrong-name".into()),
            ..Default::default()
        });
        let snap = ObsSessionContext {
            session_id: Some("bookmark-a".into()),
            session_name: Some("alpha".into()),
            ..Default::default()
        };
        let p = langfuse_session_properties_from(&snap);
        assert_eq!(
            p,
            vec![
                ("langfuse.session.id".into(), "bookmark-a".into()),
                ("xylitol.session.id".into(), "bookmark-a".into()),
                (
                    "langfuse.trace.metadata.session_name".into(),
                    "alpha".into()
                ),
            ]
        );
        let g = langfuse_generation_properties_from("gpt-test", &snap);
        assert!(g.contains(&("langfuse.session.id".into(), "bookmark-a".into())));
        assert!(g.contains(&("xylitol.session.id".into(), "bookmark-a".into())));
        assert!(
            !g.iter()
                .any(|(k, _)| k == "xylitol.session.llm_gateway_session_id")
        );
        assert!(!g.iter().any(|(_, v)| v == "process-wrong"));
    }

    #[test]
    fn properties_omit_unpresented_gateway_and_missing_fork_edge() {
        let snap = ObsSessionContext {
            session_id: Some("s1".into()),
            parent_session_id: Some("parent".into()),
            fork_at_entry_id: Some("u6".into()),
            llm_gateway_session_id: Some("gw".into()),
            ..Default::default()
        };
        let p = langfuse_session_properties_from(&snap);
        assert!(p.contains(&("xylitol.session.parent_session_id".into(), "parent".into())));
        assert!(p.contains(&("xylitol.session.fork_at_entry_id".into(), "u6".into())));
        assert!(p.contains(&("xylitol.session.llm_gateway_session_id".into(), "gw".into())));
        let bare = ObsSessionContext {
            session_id: Some("s1".into()),
            ..Default::default()
        };
        let p2 = langfuse_session_properties_from(&bare);
        assert!(
            !p2.iter()
                .any(|(k, _)| k.starts_with("xylitol.session.parent"))
        );
        assert!(
            !p2.iter()
                .any(|(k, _)| k == "xylitol.session.llm_gateway_session_id")
        );
        assert!(
            !p2.iter()
                .any(|(k, _)| k == "xylitol.session.fork_at_entry_id")
        );
    }
}
