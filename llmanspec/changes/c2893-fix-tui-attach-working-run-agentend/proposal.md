---
depends_on: []
---

# TUI attach 第二轮 Working 空转（写者租约 / AgentEnd）

## Why

c2890 本地 mermaid 手测时，session `c6a98522-1a5d-44ee-8dd8-54a4f607ec31` 第二句「你能给我一个故意出错的mermaid吗」TUI 一直 Working、助手体空，llamacpp 侧已空闲。当时怀疑 mermaid-text 画图卡住。日志把这条从 c2890 里拆开：不是盒线渲染，是产品 TUI attach Host 的写者租约与远程 `run()` 收口。

**证据（2026-10-11，`$HOME/.xylitol`）：**

1. jsonl 只有第一轮 user + assistant；第二句从未落盘。
2. `provider-trace` 最近只有 `58ed8674`（第一轮 mermaid 展示，text-only + `Done`）。llamacpp「对话完毕」是这一轮，不是第二句返回了空。
3. 05:50:00Z 第一轮：`tool freeze gate armed` → `XyDriver::run after tool freeze prompt_len=561` → `run_ensure_session 0ms`。05:50:39Z 该 TUI 进程停。
4. 05:52:08Z 再 attach 同一 session。05:52:17Z `XyDriver::run starting prompt_len=43`（tools 已 frozen 路径）、`host_run_await 0ms`（远程 stream 懒构造）。之后 **没有** `run_ensure_session` / 新 request。
5. `host_tick` 全程 0ms（约 16ms busy ticker）直到 05:55:19Z Esc。UI 循环活着，不是 mermaid-text 把绘制打死。
6. Esc：`ClearQueue` → `writer_conflict: another client is the writer for this session`。产品 TUI 默认 attach 本机 Host；写者租约 sticky，断连不释放。
7. 远程 `XyRemoteDriver::run` 只把 `XyEvent::AgentEnd` 当结束。`Error` / Busy / spawn 静默 return 都不会拆 Working。`begin_run` 在本地立刻打 Working，所以用户看见转圈、没有 Error 行。

本票 **延后**：c2890 mermaid 先收口。手测时不要对仍在听的 Host 再开第二个 `xylitol tui --session`。

## What Changes（propose 时落地）

- 写者租约：mux / TUI 断连 MUST 释放该 session 的 writer token，否则下一客户端 prompt 不是 writer。
- 远程 `run()`：`XyEvent::Error`（含 Busy / unary prompt 失败）MUST 结束这条 stream，不得只等 `AgentEnd`。
- Esc abort / `ClearQueue`：对本客户端已取得或正在等的 run，MUST NOT 因 sticky 他者租约而 `writer_conflict` 静默。
- Host `METHOD_PROMPT` spawn：`driver` 为 `None` 时 MUST 下行 Error，禁止 unary 已 Ok、spawn 直接 `return`、客户端永远等 `AgentEnd`。
- Working 空转 MUST 能在 Error / abort 收口到 Idle；不得只留 spinner。

## 非目标（草案）

- **不实施**（仅 proposal，c2890 之后再 propose）。
- 不改 mermaid 围栏画图（c2890）。
- 不放开多客户端同时写同一 session。

## Capabilities（propose 时）

- `app-tui-*` / `server-*` 会话写者与下行收口（具体 capability 正式化时再钉）

## Impact

- 复测：同一 Host，TUI 退出再 `--session` 续上，第二句必须出正文或可见 Error，不得永久 Working。
- 与 c2842（session-scoped 命令身份）、c2849（arm_tool_freeze 租约竞态）同类，但是 **断连释放 + AgentEnd 唯一收口**，不是 `/model` 打错槽。

## Further Notes

- 观测：`just obs-summary` / `obs-requests` / `obs-tui-lag`；session 目录 `{agent_dir}/sessions/<id>/active-0.jsonl`。不要整文件 Read 大 jsonl。
- 远程 `run()` 结束条件：`src/app/core/driver/remote.rs`（`is_end = matches!(ev, XyEvent::AgentEnd { .. })`）。
- 租约：`src/app/server/host.rs` `take_writer_lease` / `WriterLease`；订阅 `METHOD_SUBSCRIBE` **不** mint token，prompt 才 mint。
- unary prompt 30s 超时；本次 3 分钟 Working 且无 `prompt failed` / `XyEvent::Error` WARN → unary 已 Ok，Host 未进 `InProcessDriver::run`，TUI 在等永远不到的 `AgentEnd`。
