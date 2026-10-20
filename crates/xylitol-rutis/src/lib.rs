//! Unique `rutis` import boundary (assembly adapter).
//!
//! Official plugins, `xylitol-runtime`, and product surfaces must not
//! `use rutis`. The composition root talks to this crate instead.

use std::sync::Arc;

pub use rutis::{BoxFuture, CordisError, Ctx, Effect, Plugin, TypeKey};

/// Process fiber root. Call from the composition root inside Tokio.
/// Do not construct at crate load: an unused `Ctx` still owns the registry,
/// event bus, and fiber table.
pub fn process_root() -> Result<Ctx, CordisError> {
    Ctx::root()
}

/// Mount `plugin` and wait until it is Active (or Failed).
pub async fn apply(ctx: &Ctx, plugin: impl Plugin) -> Result<(), Arc<CordisError>> {
    ctx.plugin(plugin).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn process_root_constructs_inside_tokio() {
        let ctx = process_root().expect("root ctx");
        ctx.shutdown().await.expect("shutdown");
    }

    struct Marker;

    impl Plugin for Marker {
        fn name(&self) -> &str {
            "marker"
        }

        fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
            Box::pin(async move {
                ctx.provide(())?;
                Ok(Effect::Done)
            })
        }
    }

    #[tokio::test]
    async fn apply_waits_until_active() {
        let ctx = process_root().expect("root ctx");
        apply(&ctx, Marker).await.expect("apply");
        assert!(ctx.get::<()>().is_some());
        ctx.shutdown().await.expect("shutdown");
    }
}
