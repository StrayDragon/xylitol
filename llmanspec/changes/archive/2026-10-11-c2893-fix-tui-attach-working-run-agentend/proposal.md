---
depends_on: []
branch: sdd/c2893-fix-tui-attach-working-run-agentend
base_branch: main
base_sha: 048738c5d8e568e739ff5718b9e7a7d4f4997f09
---

# 砍协议写者令牌，会话占用改内核锁

## Why

产品 TUI attach 本机 Host 后，退出再 `--session` 同一会话会永久 Working。实机 session `c6a98522-1a5d-44ee-8dd8-54a4f607ec31`：第二句未落盘，Esc 清队列 `writer_conflict`。根因不是 mermaid，是 r1793 防双写落在 **Host 内存 UUID**：只 mint、连接死了不还；`run()` 又把业务错误当 prompt 成功，死等 AgentEnd。

对照 `../deepseek-harness`（session 目录内核 flock，进程死 fd 关锁没）和 `../maka`（State Root OS lock）。个人 harness 要的是「同一 session 同时一个写者」，不是信封令牌。JSONL 模块还写着 file locking，实现里没有。

## What Changes

- 砍协议 `writer_token` / `X-Writer-Token`。跨进程占用 = 内核排他锁，对齐 dsh：会话目录 **`session.lock`**（POSIX `flock`；Windows 由该路径派生命名信号量，不锁数据文件）。还锁 = close fd / 关 handle，不 unlink。
- 该 session 最后一条订阅连接干净退出（TUI Ctrl+C / WS Close / 切走 session）立刻还；异常断开 linger（可配置，默认 60s，`0` 立即还）。linger 内同进程重连续占；他进程争用大声失败，不抢活锁。Host 进程死由内核还。
- 同进程第二客户端靠 `run_inflight`。纯 HTTP 无订阅：回合结束且订阅者为 0 则 linger。
- `run()` 必须看 unary 业务成败；未准入结束 stream。Busy + 空 follow-up 的非中止 Error → Idle。
- `arm_tool_freeze` 仍不因写占用失败而卡死首轮门。
- 破坏性：旧客户端不能再靠 header 令牌当写者身份。Pre-0.0.1，无迁移脚本。

## 非目标

- mermaid、通知栈、滚动槽。
- opencode stale 抢锁、last-writer-wins、Maka State Root / Epoch。
- 每次 append 才加锁。

## Capabilities

- `server-core`（改 r1793；顺带 r1785 / r1904 / r1925 去令牌措辞）
- `agent-session-store`（持久化层内核写占用）
- `app-tui-bridge`（Busy + Error 收口 Idle）
- `protocol-app`（queue_stats 场景不再断言令牌）

## Impact

- 同一 Host：TUI 退出再 `--session`，第二句出正文或可见错误，不得永久 Working。
- 两个 Host 进程写同一会话：后到者失败，不得撕 JSONL。
- 既有 `writer-lease` / `writer-token-via-ws-unary` BDD 改写为占用/释放/冲突，不再要 header 令牌。

## Open Questions

- **[已定]** 砍 UUID，改内核锁。
- **[已定]** 锁 `session.lock`（不锁 manifest）；POSIX flock + inode 核对；Windows 命名信号量。还锁 = close，不 unlink。
- **[已定]** 干净最后连接立刻还；异常断开 linger 默认 60s；不抢他进程活锁。

## Further Notes

- 设计：同目录 `design.md`。调研：`research/writer-lease-lifetime.md`、`research/peer-writer-exclusion.md`。
- 同 Host 双 TUI resume 同一 session 会扇出打字机（占用是写引擎，不是连接）。产品未决，草案见 `c2895-update-same-host-second-attach`。
