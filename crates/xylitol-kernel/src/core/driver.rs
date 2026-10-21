//! Core plugin: `XyDriver` façade (protocol-agnostic).
//!
//! `XyComposedDriver` is the in-process implementation assembled after this
//! fiber is Active. Per-token deltas stay on `XyDriver::run`; they do not
//! enter the rutis bus.

use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub struct DriverMarker;

pub struct DriverPlugin;

impl Plugin for DriverPlugin {
    fn name(&self) -> &str {
        "driver"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(DriverMarker)?;
            Ok(Effect::Done)
        })
    }
}
