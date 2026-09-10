//! Agent layer — thin orchestration over the runtime domain.
//!
//! ## 入口（库用户从这里进门）
//!
//! - [`AgentRuntime`]（[`runtime::AgentRuntime`]）：会话绑定的 ReAct actor。
//!   交互层（cli/rpc/server/tui）经 `XyDriver` 持有它，调用
//!   `bind_session` / `submit_root` / `abort` / `set_tools` 等方法。
//! - [`AgentCapabilities`]（[`capabilities::AgentCapabilities`]）：可插拔的能力聚合体
//!   （models + io + tools + 编排状态）。被 `AgentRuntime` 持有；构造期由
//!   [`AgentBuilder`] 装配。
//! - [`RuntimePorts`]（[`runtime::RuntimePorts`]）：可克隆的构造基线；
//!   `AgentBuilder::build_ports` → `materialize_runtime` 物化隔离 actor
//!   （未来 sub-agent factory 接缝；不共享 history / cancel / active turn）。
//!
//! 两者关系是 Runtime + Capabilities：`AgentRuntime` 以单写者方式跑 ReAct，
//! `AgentCapabilities` 是被驱动的能力体。
//!
//! ## 分层约束
//!
//! 交互代码应只从本 mod 级（`crate::agent::*`）import。直接 reach into
//! `agent::runtime` / `agent::capabilities` / `agent::tools` 子模块是分层违规，唯一
//! 例外是组合根（`app::core::composition`），它在构造期注入具体 adapter。
//! [`AgentRuntime`] 是 in-process 半边的 XyDriver 抽象（见 c265）；远程半边是
//! Host 四象限信封（`app::server` POST unary + mux 下行）。

pub mod builder;
pub mod capabilities;
pub mod compaction;
pub mod context_policy;
pub mod llm_project;
pub mod model;
pub mod prompt;
pub mod runtime;
pub mod tools;

// ── 公共入口（mod 级 re-export）──────────────────────────────────
// 库用户应从 `crate::agent::*` import，而非 reach into 子模块。

pub use crate::agent::builder::AgentBuilder;
/// Shared capability vocabulary (queue/session stats and modes, model
/// registry, hook-block error). The `capabilities::AgentCapabilities`
/// aggregate itself stays crate-internal — `AgentRuntime` is the only
/// orchestration entry.
pub use crate::agent::capabilities::{
    HookBlockedError, ModelRegistry, QueueMode, QueueStats, SessionStats,
};
/// ReAct 循环运行时（驱动 [`AgentCapabilities`]）。
pub use crate::agent::runtime::AgentRuntime;
/// Clonable construction baseline for materializing isolated [`AgentRuntime`]s.
pub use crate::agent::runtime::RuntimePorts;
#[cfg(test)]
pub use crate::agent::runtime::XyEventStream;
pub use crate::agent::runtime::hooks::max_turns_stop_hook;
pub use crate::agent::tools::MCP_FIRST_TURN_GATE_TIMEOUT;
/// Semantic ownership: agent re-exports shared protocol vocabulary.
pub use crate::protocol::lifecycle::XyEvent;
