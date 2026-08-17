# Tasks: c2250-harden-session-write-path

## 1. Branch binding 与 Specs landing

- [x] 1.1 前置：本规划壳（proposal/design/tasks/research）已提交到默认分支，工作树干净；然后 `llman sdd change start c2250-harden-session-write-path`
- [x] 1.2 [blocked-by: 1.1] 按 design §3 文本改写 `llmanspec/specs/agent-session-store/spec.toon` 的 s7（替换「原子 append 与文件锁」为崩溃原子性 + 单写者假设），补对应非执行场景行
- [x] 1.3 [blocked-by: 1.2] `llman sdd validate c2250-harden-session-write-path --strict --no-interactive`；`llman sdd show c2250-harden-session-write-path --json` 确认 `readyToImplement=true`

## 2. 原子重写

- [x] 2.1 [blocked-by: 1.3] `write_entries_to_disk`（`src/infra/session/manager.rs:133`）改 temp 写入 + `sync_all` + `rename`；失败清理 `*.tmp`；三条触发路径（deferred flush / flush merge / header repair）不单独改
- [x] 2.2 [blocked-by: 2.1] 单测：既有文件重写后内容正确且无 `*.tmp` 残留；新建文件分支同样经 rename；tmp 写失败 / rename 失败时原文件字节不变

## 3. mutator async 化与错误可见

- [x] 3.1 [blocked-by: 2.2] `AgentCapabilities::{select_model, select_model_with_source, set_thinking_level, cycle_thinking_level}` async 化；删 `tokio::spawn`，直接 await `append_session_entry`；失败 `log::warn!`（target `xylitol::session`，带 kind）
- [x] 3.2 [blocked-by: 3.1] 转发链跟进：`src/agent/runtime/react/mod.rs:263-283`、`XyDriver` trait 三方法（`src/app/core/driver/proto.rs:49-66`）→ `async fn`、in_process / remote / harness 三实现、`src/app/core/dispatch.rs:127,139`、`src/app/core/bootstrap.rs:723,742`、`src/app/tui/effects/pending_ui.rs:215,223`
- [x] 3.3 [blocked-by: 3.1] `src/agent/runtime/react/support.rs:109` 的 `let _ =` → 失败 `log::warn!`
- [x] 3.4 [blocked-by: 3.2] 排序单测：`select_model` 后紧接一轮 run，JSONL 中 modelChange 行先于该轮 assistant 行
- [x] 3.5 [blocked-by: 3.2] 受波及测试调用点补 `.await`（`rg '\bselect_model\(|set_thinking_level\(|cycle_thinking_level\(' src/` 约 40 处；同步测试上下文改 `#[tokio::test]` 或 `block_on`，跟随仓库习惯）

## 4. 闸

- [x] 4.1 [blocked-by: 3.3, 3.4] `just fmt` + `just lint` + `just test` + `just test-tui`
- [x] 4.2 [blocked-by: 4.1] `llman-sdd-verify` 出报告；全绿后 `llman sdd change finalize c2250-harden-session-write-path`
