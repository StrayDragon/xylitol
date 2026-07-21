//! Agent layer — thin orchestration over the runtime domain.
//!
//! ## 入口（库用户从这里进门）
//!
//! - [`AgentRuntime`]（[`runtime::AgentRuntime`]）：ReAct 循环运行时，驱动
//!   [`AgentCapabilities`] 跑 turn。交互层（cli/rpc/server/tui）经 `XyDriver` 持有它，
//!   调用 `run` / `abort` / `set_tools` 等方法。
//! - [`AgentCapabilities`]（[`session::AgentCapabilities`]）：可插拔的能力聚合体
//!   （models + io + tools + 编排状态）。被 `AgentRuntime` 持有；构造期由
//!   [`AgentBuilder`] 装配。**不是** [`crate::protocol::message::AgentContext`]
//!   （那是 LLM 请求快照）。
//!
//! 两者关系是 Runtime + Capabilities：`AgentRuntime` 跑 ReAct 循环，
//! `AgentCapabilities` 是被驱动的能力体。
//!
//! ## 分层约束
//!
//! 交互代码应只从本 mod 级（`crate::agent::*`）import。直接 reach into
//! `agent::runtime` / `agent::session` / `agent::tools` 子模块是分层违规，唯一
//! 例外是组合根（`app::core::composition`），它在构造期注入具体 adapter。
//! [`AgentRuntime`] 是 in-process 半边的 XyDriver 抽象（见 c265）；远程半边是
//! `app::server::ws` / `app::server::rest`。

pub mod builder;
pub mod compaction;
pub mod llm_project;
pub mod model;
pub mod prompt;
pub mod runtime;
pub mod session;
pub mod text;
pub mod tool_result_quiet;
pub mod tools;

// ── 公共入口（mod 级 re-export）──────────────────────────────────
// 库用户应从 `crate::agent::*` import，而非 reach into 子模块。

pub use crate::agent::builder::AgentBuilder;
pub use crate::agent::llm_project::project_for_llm;
/// ReAct 循环运行时（驱动 [`AgentCapabilities`]）。
pub use crate::agent::runtime::AgentRuntime;
pub use crate::agent::runtime::hooks::BeforeToolHook;
pub use crate::agent::runtime::hooks::{ShouldStopAfterTurnCtx, ShouldStopAfterTurnHook};
pub use crate::agent::runtime::{AgentHooks, XyEventStream};
pub use crate::agent::session::AgentCapabilities;
pub use crate::agent::session::{PendingMessageQueue, QueueMode, QueueStats};
/// Semantic ownership: agent re-exports shared protocol vocabulary.
pub use crate::protocol::lifecycle::XyEvent;
pub use crate::protocol::message::AgentMessage;
