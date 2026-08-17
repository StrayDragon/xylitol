# 调研：session 写入路径（2026-08-17）

结论与决策依据的原始证据。行号基于 main `31618ae3`；执行时以符号为准复核。

## 1. 非原子重写的三条触发路径

`write_entries_to_disk`（`src/infra/session/manager.rs:133-156`）：序列化全部条目 → `tokio::fs::write` 直接覆盖。调用点：

| # | 路径 | 位置 | 危害 |
|---|---|---|---|
| a | 新会话 deferred flush（首条 assistant 建文件） | `append` → `manager.rs:399` → `flush_pending_to_disk`（`:158-179`）else 分支 `:177` | 崩溃丢新会话早期记录（低危） |
| b | flush merge 重写既有文件 | `:168-175`：文件已存在（fork 子会话 body 已由 `append_with_id` 写入、header 仍 pending，见 `:1205`；或 bind-then-`/model` 竞态）→ 读全文件 + `merge_pending_ahead_of_disk`（`:182-214`）+ 覆盖重写 | **唯一能损坏既有数据的路径**（fork 常规路径） |
| c | `create()` header repair | `:286-296`：无 header 的既有文件前插 header 后全量重写 | 同 b，触发罕见 |

append 路径（`manager.rs:352-374`）：`OpenOptions::append` + 单行 `write_all`，相对安全，不动。fork body 复制走 `append_with_id`（`:503-548`），append，不经重写路径。

## 2. fire-and-forget 落盘

- `persist_thinking_level_change`（`src/agent/capabilities/model_ops.rs:73-103`）与 `select_model_with_source`（`:133-158`）：同步 `&mut self` 不能 await → `tokio::spawn` + `let _ =` 丢弃结果。
- 风险实测依据：
  1. 退出竞态：`/model`、`/thinking` 后立即 `:q` 或 print 模式跑完即退 → spawn 未执行完丢行 → resume 后 model/thinking 恢复错（thinking 有 `restore_thinking_level`，`model_ops.rs:123-125`）；
  2. 吞错：append 失败无日志无事件，违反根 AGENTS / src AGENTS 观测规则（失败日志带 error.kind）；
  3. 排序不定：spawn 的 append 与后续 awaited append 竞争；load 时 leaf 重置为文件最后一行 id，modelChange 落在最后一条 assistant 之后会让 leaf 指向 modelChange（parentId 链仍通，影响小但语义偏移）。
- `persist_agent_message_with_thought_elapsed`（`src/agent/runtime/react/support.rs:78-110`）：async 且被正确 await，仅 `:109` 的 `let _ =` 吞 append 错误。

## 3. async 化波及面（已核查）

- `XyDriver` trait 已 `#[async_trait]`（`src/app/core/driver/proto.rs:27`）；目标三方法 `proto.rs:49,57,66`。
- 实现：in_process `src/app/core/driver/in_process.rs:537-605`；remote `src/app/core/driver/remote.rs:295-384`（RPC 客户端，async 上下文）；harness `src/app/tui/harness.rs:503-539`。
- 调用点：`src/app/core/dispatch.rs:127,139`；`src/app/core/bootstrap.rs:723,742`；`src/app/tui/effects/pending_ui.rs:215,223`。
- 测试调用点约 40 处（`src/agent/model/manager.rs`、`src/agent/runtime/react/tests.rs`、`src/app/core/driver/in_process.rs` 内联测、`src/app/tui/harness.rs` 内联测等）。

## 4. tmp+rename 既有先例（模式对齐，不新造）

- trust：`src/infra/trust/store.rs:166-178`（`trust.json.tmp` + rename；锁文件为另一机制 O_EXCL + 重试，本票不引入锁）
- settings：`src/infra/settings/storage.rs:73-105`（`settings.json.tmp` + rename）
- tokenizer：`packages/xylitol-ai-bridge/src/tokenize/mod.rs:219-255`（`<file>.tmp` + rename）

## 5. s7 原文（将被替换）

`llmanspec/specs/agent-session-store/spec.toon:14`：

> s7,文件操作,SessionManager MUST 使用原子 append（仅追加 JSONL）与文件锁保证并发安全。

与代码不符的两点：无文件锁（Cargo 无 fs2/fd-lock，manager 无锁原语）；「仅追加」被 §1 的重写路径否定。

## 6. 用户决策记录（2026-08-17 会话）

- 首个预期版本不考虑跨进程并发使用方式 → 不做文件锁，s7 改写为真实保证而非保留误导性条款。
- 崩溃原子性（temp+rename）要做：fork 常规路径会重写既有文件，非理论问题。
- fire-and-forget：不惜成本一步到位 async 化（不做退出 drain 屏障）。
- 与 `c2260-bump-session-format-v6` 的关系：本票先做（A 的 version bump 落盘走本票修好的路径），c2260 depends_on 本票。
