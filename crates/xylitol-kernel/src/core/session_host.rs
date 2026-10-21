//! Core plugin: session instance factory + single-writer lease.
//!
//! Each product session is a rutis instance subtree (`TypeKey::instance`).
//! Global services stay on the root ctx.

use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

/// Handle for creating/removing per-session instance subtrees.
pub struct SessionHost;

pub struct SessionHostPlugin;

impl Plugin for SessionHostPlugin {
    fn name(&self) -> &str {
        "session-host"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(SessionHost)?;
            Ok(Effect::Done)
        })
    }
}
