//! Current xylitol session identity for Langfuse OTEL attributes (c1480).
//!
//! Resolution (Keybindings-style) for **idle** paths (no generate options):
//! 1. thread-local [`ObsSessionScope`] if entered (tests / sync inject)
//! 2. else process `Mutex` slot (production — survives tokio worker hops)
//!
//! Materialized sessions MUST derive identity from the owning runtime's facts
//! and MUST NOT read this slot. The process slot is a fallback for optionless
//! paths and tests.

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

use crate::generate::{
    ObsSessionContext, langfuse_observation_properties_from, langfuse_session_properties_from,
};

fn slot() -> &'static Mutex<ObsSessionContext> {
    static SLOT: OnceLock<Mutex<ObsSessionContext>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(ObsSessionContext::default()))
}

thread_local! {
    /// Tests / sync inject: prefer this over the process slot while the guard lives.
    static SCOPED: RefCell<Option<ObsSessionContext>> = const { RefCell::new(None) };
}

/// RAII install of a thread-local obs session context.
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

/// Idle-path session properties (process slot / TLS). Generate MUST use
/// [`langfuse_session_properties_from`] with the options snapshot instead.
pub fn langfuse_session_properties() -> Vec<(String, String)> {
    langfuse_session_properties_from(&obs_session_context())
}

/// Idle-path observation properties (process slot / TLS).
pub fn langfuse_observation_properties(observation_type: &str) -> Vec<(String, String)> {
    langfuse_observation_properties_from(observation_type, &obs_session_context())
}

/// Generation helpers from an explicit session snapshot (overlapping generate).
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
