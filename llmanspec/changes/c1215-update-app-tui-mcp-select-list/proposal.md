---
change_id: c1215-update-app-tui-mcp-select-list
title: /mcp 改为 SelectList（对齐 model/resume）并优化开槽性能
status: designed
priority: 1215
depends_on:
- c1210-update-mcp-hot-merge-ungate
author: agent
branch: sdd/c1215-update-app-tui-mcp-select-list
base_sha: a7fcca3f56e217eb823ce5dfaf3e8fde89afbf9e
checkpointed: false
---

# c1215 — /mcp SelectList + open perf

> Design SSOT：[`src/app/tui/design/mcp-input-cue.md`](../../../src/app/tui/design/mcp-input-cue.md) · playground `?slot=mcp-cue`。

## Why

c1210 的 `/mcp` 是只读 `Vec<String>`：无法 ↑↓ 选中，开槽每次 `await loaded_resources_snapshot()` 易卡。后续「本 session 临时关某 MCP」需要可选项缝；现在升 SelectList 壳 + 缓存开槽。

## 已拍决策

| 项 | 决定 |
|---|---|
| 组件 | 包 `SelectList`；↑↓ + reverse 焦点；对齐 `/model` / `/session-resume` |
| Enter（本波） | **关槽**（真 toggle 后置） |
| Esc | 关槽；不 abort agent |
| Perf | 优先用 host 已缓存的最近 `LoadedResourcesSnapshot`；缺缓存再 await |
| 短 cue | 保持右对齐 `mcp pending (see /mcp)`（c1210） |
| 非目标 | session 级 disable MCP；单 server reload |

## What Changes

- 改写 `atm17`（及必要 ath27）：SelectList 交互 + Enter 关槽 + 缓存开槽
- 实现：`mcp_list: SelectList`、mount/render/input；去掉纯文本行渲染
- OpenMcp：先挂缓存 snap，必要时再刷
- harness：↑↓ 选中可观测；Enter/Esc 关；缓存路径不重复阻塞（可测缝）

## Capabilities

- `app-tui-commands`（atm17）
- `app-tui-host`（ath27 开槽/缓存，若需）

## Impact

- `/mcp` 手感对齐其它 picker；为 toggle 留缝
- 开槽更轻

## Out of scope

- 真·临时禁用 MCP
- c1220 prompt 模板
