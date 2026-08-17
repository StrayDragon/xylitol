//! Current xylitol session identity for Langfuse OTEL attributes (c1480).
//!
//! Lives in ai-bridge so both `llm.request` and agent ReAct obs can read
//! it without agent → infra edges. App driver updates on session switch/name.
//!
//! Resolution (Keybindings-style):
//! 1. thread-local [`ObsSessionScope`] if entered (tests / sync inject)
//! 2. else process `Mutex` slot (production — survives tokio worker hops)
//!
//! Production MUST keep writing the process slot (no prod TLS-only scope around
//! HTTP middleware). Scope is for cargo-test isolation.

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObsSessionContext {
    pub session_id: Option<String>,
    pub session_name: Option<String>,
}

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

/// Run `f` under an [`ObsSessionScope`].
pub fn with_obs_session<R>(ctx: ObsSessionContext, f: impl FnOnce() -> R) -> R {
    let _g = ObsSessionScope::enter(ctx);
    f()
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

/// Fastrace property pairs for Langfuse session mapping.
pub fn langfuse_session_properties() -> Vec<(String, String)> {
    let ctx = obs_session_context();
    let mut out = Vec::with_capacity(2);
    if let Some(id) = ctx.session_id.filter(|s| !s.is_empty()) {
        out.push(("langfuse.session.id".into(), id));
    }
    if let Some(name) = ctx.session_name.filter(|s| !s.is_empty()) {
        out.push(("langfuse.trace.metadata.session_name".into(), name));
    }
    out
}

/// Append Langfuse observation type (+ session + llm obs lane) onto span property lists.
pub fn langfuse_observation_properties(observation_type: &str) -> Vec<(String, String)> {
    let mut out = vec![(
        "langfuse.observation.type".into(),
        observation_type.to_string(),
    )];
    out.extend(langfuse_session_properties());
    out.extend(xylitol_obs_lane_properties(XYLITOL_OBS_LANE_LLM));
    out
}

/// Fastrace attribute for Collector / consumer routing (`llm` | `infra`).
pub const XYLITOL_OBS_LANE_ATTR: &str = "xylitol.obs.lane";
/// LLM / agent product process-tree lane.
pub const XYLITOL_OBS_LANE_LLM: &str = "llm";
/// Infra / client failure-experience lane (future spans; prepare-fail MUST NOT fake LLM spans).
pub const XYLITOL_OBS_LANE_INFRA: &str = "infra";

/// Property pairs for `xylitol.obs.lane`.
pub fn xylitol_obs_lane_properties(lane: &str) -> Vec<(String, String)> {
    vec![(XYLITOL_OBS_LANE_ATTR.into(), lane.to_string())]
}

/// Generation helpers: observation type + single model attribute.
///
/// Prefer `langfuse.observation.model.name` only (Langfuse OTEL mapping; `langfuse.*`
/// takes precedence). Do not also set bare `model` / `gen_ai.request.model` — same
/// mapped field, and bare `model` can force generation typing on unrelated spans.
pub fn langfuse_generation_properties(model: &str) -> Vec<(String, String)> {
    let mut out = langfuse_observation_properties("generation");
    if !model.is_empty() {
        out.push(("langfuse.observation.model.name".into(), model.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_without_name() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-1", None);
        let p = langfuse_session_properties();
        assert_eq!(p, vec![("langfuse.session.id".into(), "sid-1".into())]);
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
        assert_eq!(p3, vec![("langfuse.session.id".into(), "sid-2".into())]);
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
        assert!(p.contains(&("langfuse.session.id".into(), "sid-g".into())));
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
}
