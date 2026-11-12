---
depends_on: []
title: "Fix TUI host busy state and ask slot"
branch: sdd/c3275-fix-tui-host-busy-ask
base_branch: main
base_sha: 2caaa7076abe8be033d0776d8c2fd13fe556c439
---

# TUI 端 busy 状态与 ask 单槽修复

## Why

app-tui host 层的忙闲状态由 4 个写点 × 2 个载体拼出，ask 提问槽可被静默覆盖（Lead 已复核 host/mod.rs:440-615、:960-1000 原文）：

- **busy 多源**（D1）：run_active/bash_active/reload_active 三个 bool + UiModel.phase；end_bash_exec(:589) 只回查 run_active 不回查 reload_active——reload 中结束 bash 会把 phase 置 Idle，状态栏丢 Reloading；note_user_abort 清 run/bash 但不动 reload_active；set_busy_status 又任意事件置 Busy。
- **ask 单槽覆盖**（D4，已复核 ask_host.rs:39-105）：push_from_server 直接 *slot = Some(...) 不检查占用；远端 approval 到达时覆盖本地 ask，旧 oneshot drop，调用方只见 Aborted 且无日志。
- **流式缓冲六文件写**（D3）：streaming_assistant/streaming_thinking/think_id/current_role/thought_clock 在 6 个文件手工清，新事件族漏清即幽灵流式块。
- **PendingOps 14 lane**（D5）：语义靠 effects drain 顺序而非类型，abort 与 dequeue 可同真。
- **结算标记字符串判据**（D23）：run_applied_token_settlement 依 reason == "turn_settled"|"after_compaction" 字符串。

## What Changes

- busy 收敛单一来源（HostBusyPhase 枚举：Idle/AgentRun/Bash/Reload，交叉组合显式表达），end_* 全部走同一收口函数按当前相位决定是否回 Idle。
- ask 槽占用检查 + 覆盖策略显式化（拒绝新请求或排队），覆盖时日志可观测。
- 流式缓冲收敛单一 reset/flush 入口（事件驱动的 state struct）。
- PendingOps lane 类型化（每 lane 独立 take 已有，补互斥约束与配对清理）；结算判据改枚举。

## Capabilities

- app-tui-host（host 驱动引擎）；app-tui-ask（ask 槽）；app-tui-bridge（流式缓冲）。

## Impact

- 范围：crates/xylitol-app-tui/src/{host,ask_host,bridge}。
- 验证：reload 中 bash 结束的状态栏 harness 断言；远端 approval + 本地 ask 并发场景；流式缓冲重置矩阵。

## 复核与范围裁定（2026-11-12）

- busy 多源 / reload 中 end_bash 误置 Idle / abort 不清 reload / ask 单槽覆盖 / 结算字符串判据：均复核成立（证据见 design §7）。
- 「流式缓冲六文件手工清」部分不成立：`UiModel::clear_streaming_buffers()` 已是单一复位入口，缺口只在 lifecycle 与 session_tree 两处绕过。
- PendingOps 14 lane 类型化：design §1 已保留 lane 机制，本 change 不实施（另单）。

## Open Questions

1. ask 槽并发策略：后到拒绝（推荐，提示用户先答当前）还是排队？
2. busy 相位枚举放 host 层还是 bridge UiModel（单一真值选边）？
