//! Cross-surface seams shared by all application entry points.
//!
//! Everything under `app/core/` is the **shared seam layer**: privileged
//! modules that more than one surface depends on. They are distinct from the
//! self-contained surface directories (`cli/`, `rpc.rs`, `server/`, `tui/`,
//! `gui.rs`) and are the only `app/` modules allowed to import `agent` and/or
//! `infra`.
//!
//! - [`composition`] — the composition root: the single module permitted
//!   to import both `agent` and `infra`, centralizing Agent wiring so CLI/RPC/
//!   Server/TUI never duplicate it.
//! - [`driver`] — the runtime boundary (`Driver` trait + `InProcessDriver`/
//!   `RemoteDriver`). It imports `agent` (mod-level) only (never `infra`, per
//!   la11); every surface depends on this, never on `agent` internals.

pub(crate) mod composition;
pub(crate) mod driver;
