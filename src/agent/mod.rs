//! Agent layer — thin orchestration over the runtime domain.
//!
//! ## 入口（库用户从这里进门）
//!
//! - [`ReActAgent`]（[`runtime::ReActAgent`]）：ReAct 策略驱动器，驱动 [`Agent`]
//!   跑循环。交互层（cli/rpc/server/tui）持有的就是它，调用 `run` / `abort` /
//!   `set_tools` 等方法。
//! - [`Agent`]（[`session::Agent`]）：可插拔的能力聚合体（models + io + tools +
//!   编排状态）。被 `ReActAgent` 持有；构造期由 [`AgentBuilder`] 装配。
//!
//! 两者关系是 Strategy：`ReActAgent` 是 strategy（跑 ReAct 循环），`Agent` 是
//! context（被驱动的能力体）。
//!
//! ## 分层约束
//!
//! 交互代码应只从本 mod 级（`crate::agent::*`）import。直接 reach into
//! `agent::runtime` / `agent::session` / `agent::tools` 子模块是分层违规，唯一
//! 例外是组合根（`app::core::composition`），它在构造期注入具体 adapter。
//! [`ReActAgent`] 是 in-process 半边的 Driver 抽象（见 c265）；远程半边是
//! `app::server::ws` / `app::server::rest`。

pub mod builder;
pub mod compaction;
pub mod model;
pub mod prompt;
pub mod runtime;
pub mod session;
pub mod tools;

// ── 公共入口（mod 级 re-export）──────────────────────────────────
// 库用户应从 `crate::agent::*` import，而非 reach into 子模块。

pub use crate::agent::builder::AgentBuilder;
/// ReAct 策略驱动器（跑 ReAct 循环，驱动 [`Agent`]）。
pub use crate::agent::runtime::ReActAgent;
pub use crate::agent::runtime::hooks::BeforeToolHook;
pub use crate::agent::runtime::{AgentHooks, XyEventStream};
pub use crate::agent::session::Agent;
pub use crate::agent::session::{PendingMessageQueue, QueueMode};
pub use crate::domain::lifecycle::XyEvent;
