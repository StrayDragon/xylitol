---
depends_on:
  - c95-add-bash-executor
  - c105-add-export-capabilities
  - c110-update-slash-commands
---

# c115-add-rpc-mode: RPC 模式（JSONL over stdio 全协议）

## Why
pi 的 `modes/rpc/`（~1670 LOC：jsonl.ts / rpc-mode.ts / rpc-client.ts / rpc-types.ts）是除 TUI 外主要的程序化入口，用于编辑器集成、CI、headless 自动化、SDK 客户端。xylitol 的 `--rpc` 是空壳 flag，无任何实现。本变更是 unblock headless 集成的关键（P0）。pi 的 c75-agent-session（已归档）与 c80-agent-loop（已归档）已完成，构成 RPC 派发后端。

## What Changes
- 新增 `src/interface/rpc/`：
  - `jsonl.rs`：stdin 行读取 + stdout 行写入（flush 每行）
  - `rpc_types.rs`：`RpcCommand`（对齐 rpc-types.ts 全部命令：prompt/steer/follow_up/abort/new_session/get_state/set_model/cycle_model/get_available_models/set_thinking_level/cycle_thinking_level/set_steering_mode/set_follow_up_mode/compact/set_auto_compaction/set_auto_retry/abort_retry/bash/abort_bash/get_session_stats/export_html/switch_session/fork/clone/get_fork_messages/get_last_assistant_text/set_session_name/get_messages/get_commands）+ `RpcEvent`/`RpcResponse`（含可选 `id` 用于关联）
  - `rpc_mode.rs`：主循环 `run_rpc_mode(agent_session) -> exit_code`，dispatch 到 AgentSession + 订阅 AgentEvent 流转 RpcEvent 输出
- CLI `--rpc` 接入：在 `interface/cli/mod.rs` 路由到 `run_rpc_mode`
- 信号处理：SIGINT / SIGTERM → 优雅 abort + 退出

## Capabilities
- cli-entry

## Impact
- 非破坏性：新增模块 + CLI flag 接线（`--rpc` 当前无实现，本变填补）。
- 触及文件：`src/interface/rpc/`（新增）、`src/interface/mod.rs`（导出）、`src/interface/cli/mod.rs`（路由）。
- 依赖：tokio signal（已在 features：macros/rt）。
