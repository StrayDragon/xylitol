//! Unique `rutis` import boundary (assembly adapter).
//!
//! Official plugins, `xylitol-runtime`, and product surfaces must not
//! `use rutis`. The composition root talks to this crate instead.

pub use rutis::{CordisError, Ctx, Plugin};

/// Process-wide fiber root. Call from the composition root inside Tokio.
/// Do not construct at crate load: an unused `Ctx` still owns the registry,
/// event bus, and fiber table.
pub fn process_root() -> Result<Ctx, CordisError> {
    Ctx::root()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn process_root_constructs_inside_tokio() {
        let ctx = process_root().expect("root ctx");
        ctx.shutdown().await.expect("shutdown");
    }
}
