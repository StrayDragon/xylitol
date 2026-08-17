---
depends_on: []
branch: sdd/c2250-harden-session-write-path
base_sha: 31618ae386867f9b7564b025cdb501d9d11f3699
checkpointed: true
checkpoint_sha: 31618ae386867f9b7564b025cdb501d9d11f3699
---

# session 写入路径加固：原子重写 + 变更落盘确定化

> **一句话**：会话文件全量重写路径改 temp+rename 原子替换（崩溃不截断既有内容）；model/thinking 变更落盘从 fire-and-forget 改为 await + 错误可见；spec s7 从「原子 append 与文件锁」改写为真实并发模型。
> **目的地**：写盘路径先做对，c2260 盘面 v6 的落盘走的才是安全路径。

## Why

1. **非原子全量重写**：`write_entries_to_disk`（`src/infra/session/manager.rs:133`）用 `tokio::fs::write` 直接覆盖目标文件，无 temp+rename。三条触发路径中两条会重写**既有**文件，崩溃窗口内截断既有内容：
   - 新会话 deferred flush（`manager.rs:399` → `:177`）：首条 assistant 落盘建新文件，崩溃丢该会话早期记录（低危）；
   - flush merge 重写既有文件（`manager.rs:175`）：fork 子会话常规路径——`append_with_id` 先写 body 行、header 仍在 pending，flush 时读全文件 + merge + 覆盖重写（**唯一能损坏既有数据的路径**）；
   - `create()` header repair（`manager.rs:286-296`）：无 header 的既有文件前插 header 后全量重写。
2. **fire-and-forget 落盘**：`select_model_with_source` / `persist_thinking_level_change` 是同步 `&mut self` 方法不能 await，被迫 `tokio::spawn` + `let _ =` 丢弃结果（`src/agent/capabilities/model_ops.rs:73-103,140-158`）。三个真实后果：
   - 退出竞态丢行：`/model`、`/thinking` 后立即退出（TUI `:q`、print 模式跑完即退）→ resume 后 model/thinking 恢复错；
   - append 失败（磁盘满、权限）被 `let _ =` 完全吞掉，无日志无事件，违反仓库观测规则（失败日志带 error.kind）；`persist_agent_message` 内部同样吞错（`src/agent/runtime/react/support.rs:109`）；
   - spawn 的 append 与后续 awaited 的 append 排序不定，modelChange 行可能落在最后一条 assistant 之后。
3. **spec s7 说谎**：`agent-session-store` s7 声称「MUST 使用原子 append（仅追加 JSONL）与文件锁保证并发安全」，两者都不真：无任何文件锁（Cargo 无 fs2/fd-lock），「仅追加」被上述重写路径否定。首个版本明确**不做**跨进程并发；与其留假合约误导后来者（本项目已有 REST 面与未来 Web 面规划），不如改写为真实保证。

## What Changes

1. **原子重写**：`write_entries_to_disk` 改为 temp 写入 + `sync_all` + `rename` 原子替换；失败路径清理 `*.tmp`。对齐仓库既有三处先例（trust `src/infra/trust/store.rs:166-178`、settings `src/infra/settings/storage.rs:73-105`、tokenizer `packages/xylitol-ai-bridge/src/tokenize/mod.rs:219-255`）。三条触发路径全部经此函数，一处修改全覆盖；append 路径（单行 `write_all`）不动。
2. **mutator async 化**：`AgentCapabilities::{select_model, select_model_with_source, set_thinking_level, cycle_thinking_level}` 改 async，删除 `tokio::spawn`，直接 await `append_session_entry`；失败 `log::warn!`（带 kind）。转发链跟进：`react/mod.rs:263-283`、`XyDriver` trait 三方法（`src/app/core/driver/proto.rs:49-66`，trait 已 `#[async_trait]`）、in_process / remote / harness 三实现、`dispatch.rs:127,139`、`bootstrap.rs:723,742`、TUI `pending_ui.rs:215,223`，以及约 40 处测试调用点。
3. **persist 吞错改日志**：`support.rs:109` 的 `let _ =` 改为失败 `log::warn!`。
4. **Specs landing**：改写 `agent-session-store` s7 为「崩溃原子性 + 单进程单写者假设」（确切文本见 design）。

## 非目标

- 跨进程文件锁（fs2 / fd-lock）——首个版本明确不做，s7 改写后如实声明
- 退出 drain 屏障 / shutdown 时 flush 语义
- 盘面 schema / 时间戳 / alias 清理（`c2260-bump-session-format-v6`，依赖本票）
- append 路径改写（单行 `write_all` 保持现状）
- 落盘失败升级为用户可见错误（本票只做观测日志；要不要进 `XyEvent` 是独立产品决定）

## Capabilities

- `agent-session-store`：s7 改写（写入安全条款）

## Impact

- 代码：`src/infra/session/manager.rs`、`src/agent/capabilities/model_ops.rs`、`src/agent/runtime/react/support.rs`、`src/agent/runtime/react/mod.rs`、`src/app/core/driver/{proto,in_process,remote}.rs`、`src/app/core/{dispatch,bootstrap}.rs`、`src/app/tui/effects/pending_ui.rs`、`src/app/tui/harness.rs` + 受波及测试
- 行为变化（可观察）：崩溃后不再可能出现截断的会话文件；model/thinking 变更行与后续消息行顺序确定；append 失败进入 `~/.xylitol/logs/xylitol.log` warn

## Test seams

- manager 单测：重写路径成功后内容正确且无 `*.tmp` 残留；tmp 写失败时原文件保持原样（fault 注入或只读目录模拟）
- 排序单测：`select_model` 后紧接一轮 run，JSONL 中 modelChange 行先于该轮 assistant 行
- 既有 driver / harness / TUI 测试改 async 调用（`.await`）

## Further Notes

- 调研证据（全部 file:line）：[`research/investigation.md`](./research/investigation.md)
- 后续票：`c2260-bump-session-format-v6`（depends_on 本票）
