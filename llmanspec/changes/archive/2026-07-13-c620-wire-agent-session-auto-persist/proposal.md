---
change_id: c620-wire-agent-session-auto-persist
title: "对齐 pi：稳定 session + 消息 auto-persist + 延迟落盘"
status: full
priority: 620
depends_on: []
author: agent
track: P
---

# c620-wire-agent-session-auto-persist

## Why

产品双 Esc 会话树已接 `Driver::session_tree(MessageHistory)`（c610/c615），但 `~/.xylitol/sessions/*.jsonl` 往往只有 header：ReAct 不把 user/assistant/tool 写入 store，且 `Driver::run` → `agent.run()` 每轮 `Uuid::new_v4()`，与 pi 的「一会话一 Manager + message_end 持久化」不一致。Scrollback 有字、树为 `(0/0)` 是同一根因。既有 `a8`/`as27` 已要求 auto-persist，但未落地；还需补齐稳定 sid、context 灌 history，以及 pi 式延迟落盘。

## Purpose

让 persisted session 在真实回合后可被树/travel 读取：稳定 session id、ReAct 消息写入 store、从 store 灌多轮 history，并对齐 pi 在首条 assistant 前延迟创建/刷盘。

## What Changes

1. **稳定 sid（c）**：bootstrap `session_id` 绑定 Agent；`InProcessDriver::run` 走 `run_with_id(current)`，禁止每轮新 uuid。
2. **Auto-persist（落实 a8/as27）**：user / assistant / toolResult 完成时 `append_session_entry(SessionEntry::Message{…})`（对齐 pi `message_end`）。
3. **Seed history（c）**：`run_with_id` 经 `build_session_context` + 既有转换灌入 history，再追加本轮 user。
4. **延迟落盘（b）**：persisted backend 在首条 assistant 前可只挂内存；首条 assistant 时建文件并刷出挂起条目；`in_memory` 永不写盘。
5. **共享 store**：Driver 与 Agent 使用同一 `Arc` SessionManager 实例（避免双 leaf map）。
6. **验收**：回合后 jsonl 含 message；`session_tree(MessageHistory)` 非空；产品树可看见 user 行。

## Capabilities

- `agent-session`
- `agent-session-store`
- `cli-entry`
- `app-tui-session-tree`（回归：树反映已持久化回合）

## Out of scope

- 产品 Shift+F / `fork_session` 语义选型
- 第二种 `SessionTreeKind` 真实现（`FileBrowser` 仍 Err）
- 改包 `TreeSelector` / demo 假队列形态
- CLI `--no-session` 旗标（若需要另 change；本 change 保留 `SessionManager::in_memory` 行为合约）

## Impact

- `src/agent/runtime/react.rs`、`src/agent/session/mod.rs`
- `src/infra/session/manager.rs`（deferred `_persist`）
- `src/app/core/driver.rs`、`bootstrap.rs`、`composition.rs`
- 测试：ReAct persist、同 sid 多轮、deferred flush、TUI/harness 树非空
