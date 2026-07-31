---
change_id: c1200-update-infra-mcp-startup
title: MCP 启动不挡 TTI + 进度可感（新会话与 resume）
status: designed
priority: 1200
depends_on: []
blocks: []
author: agent
---

# c1200 — MCP 启动不挡 TTI + 进度可感

> 升格自 `llmanspec/delayed-changes/c1200-…`。
> **本波**：后台/并行连接、进度进 loaded-resources、新会话与 CLI/`session-resume` 同等体验。
> **本波不做**：c1205 式 `/reload` 输入锁 / 取消 / spinner 整包 UX（reload 语义 ath20 不改；进度通道可复用，交互另案）。

## Why

启用 MCP 后，`bootstrap_mcp().await` 在进 TUI **之前**串行连 server → 新开与 `--session` resume 都卡在「黑屏等 MCP」。
`/reload` 同路径 await，像卡住——进度通道可惠及，但锁输入等属 c1205，本波不锁死。

## 已拍板

| 项 | 决定 |
|---|---|
| TTI | 有 MCP 配置也先开面（builtins）；MCP **后台**连 |
| 并行 | 多 server **并行** connect；单失败诊断、不拖死全队（mcp4） |
| 进度落点 | **主：loaded-resources 的 mcp 行**（connecting i/n · id）；**不**用滚动提示刷进度；**不**占用 agent-busy status / 下轮预告 |
| 完成/失败 | ready 后刷槽；失败摘要进 mcp 行 / diagnostics；严重失败 MAY 一条滚动提示 |
| 首轮工具 | **B**：本轮可用 builtins；MCP ready 后 **下一轮**自动带上（热合并 ToolSet） |
| 新会话 | 同上 |
| CLI `--session` resume | 先开 TUI → **尽快** rebuild transcript；MCP 与 rebuild **并行**，MUST NOT 等 MCP 完再投影历史 |
| 面内 `/session-resume` | MUST NOT 为切会话再阻塞重连 MCP（沿用已连集合；除非用户 `/reload`） |
| print | 不挡首 prompt 同等策略；进度走 log/stderr，无 TUI 槽 |
| `/reload` | 编排步骤不变；本波 **不**升格 c1205（锁 editor / 禁二次 / 取消） |

## What Changes

1. 推迟阻塞式 `bootstrap_mcp`：CLI 进 TUI/print 前不再 `await` 完整连接（或改为 spawn + 进度回调）。
2. `connect_servers` 并行化；进度经 Driver 只读缝 → host 刷新 loaded-resources。
3. Spec：`infra-mcp` 启动策略；`app-tui-host` / `app-tui-chrome` 槽进度与 resume/新会话 parity。
4. 单测 / harness：无 MCP 仍 zero-cost；有配置时 TUI 可在未连完时进入；resume rebuild 不依赖 MCP ready。

## Capabilities

- `infra-mcp`（modify）
- `app-tui-host` / `app-tui-chrome`（loaded-resources 进度）
- 视实现：`app-core` composition / driver

## Impact

- **用户**：新开与 resume 都能马上看到界面（resume 还能马上看到历史）；MCP 在头卡里长出来。
- **LLM**：首轮可能暂无 mcp: 工具；ready 后下一轮可用——可接受换 TTI。
- **风险**：用户首轮以为 MCP「没配」——mcp 行须明确 `connecting` / `configured · 0 connected` 过渡态。

## 测试边界（seam）

| Seam | 用途 |
|---|---|
| `connect_and_discover` / `McpClientManager::connect_servers` | 并行 + 诊断 |
| Driver MCP 进度/快照只读缝 + `loaded_resources_snapshot` | 槽刷新 |
| TUI host 启动 / `apply_cli_restored_session` | 新会话与 CLI resume 不挡 |
| 既有 `/reload` harness | 不回归 ath20；不新造锁交互 |

MUST 复用上述边界；MUST NOT 为进度另起脱离 feature 的 CLI。

## Out of scope

- c1205 整包 reload UX
- 改 MCP 线协议 / 新 transport
- allowlist 产品 UI
- 假进度 / 静默丢已配置 MCP
