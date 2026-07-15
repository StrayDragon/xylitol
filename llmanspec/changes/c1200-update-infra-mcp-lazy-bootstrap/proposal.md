---
change_id: c1200-update-infra-mcp-lazy-bootstrap
title: "MCP 懒加载启动 + 提示词注入点优化"
status: purpose-draft
priority: 1200
apply_band: P9-deferred
depends_on:
  - c1080-update-infra-mcp-client-product
  - c1120-add-app-tui-reload-slash
author: agent
track: R
wave: mcp-perf
domain: infra-mcp
---

# c1200-update-infra-mcp-lazy-bootstrap

## Why

启用 MCP 后，进程启动 / 首次进入 print·TUI 会在首条用户输入前同步 `connect_and_discover`，体感「很久才开始干活」。提示词侧 MCP/工具目录注入点也偏早、偏粗，不利于首屏与首轮延迟。

## Purpose（**暂缓 / deferred · 最低优先级**）

本变更 **P9 暂缓**：不挡现有产品 slash / thinking / paste 波次。等 MCP 接线与 `/reload` 稳定后再升格。

升格后目标（备忘）：

1. **懒/并行初始化**：启动不阻塞首交互；后台连接 MCP，就绪后再 merge tools；失败可诊断、不拖死主路径。
2. **注入点优化**：明确 MCP tools / 资源摘要进 system·context 的时机与形状（首轮前 vs 连接完成后热补；避免重复臃肿块）。
3. 与 `/reload`（c1120）语义兼容：显式 reload 仍可强制重连；懒加载与 idle `/reload` 不打架。

## What Changes（升格 full 时）

- `bootstrap` / `InProcessDriver::bootstrap_mcp`：同步全连 → 可选 deferred / background
- composition 缝：连接完成回调 → `set_tools` + 可选 prompt 补丁
- delta：`infra-mcp` / `app-core`（或等价 capability）+ 必要时 `agent-prompt`
- 可观测：启动日志区分「configured / connecting / connected」

## Capabilities

- `infra-mcp`（modify）
- `app-core` 或 bootstrap 缝（modify）
- 可选 `agent-prompt`（注入形状）

## Out of scope

- 本波实现（deferred）
- 改 MCP 线协议 / 新增 transport
- 替代 c1135 startup header（可见性另案）

## Ethics

- risk_level: medium
- prohibited_actions: 静默丢弃用户已配置的 MCP；连接失败当成功；把密钥写进 prompt
- required_evidence: 有 MCP 时 TTI/首轮可测下降或并行；无 MCP 路径零回归
- escalation_policy: 与 Trust / 项目 MCP 语义冲突时先对齐 bootstrap Trust 再改时机

## Depends

- c1080（产品 MCP 运行时）· c1120（`/reload` 编排，避免双重连接策略漂移）

## Notes

- 2026-07-16：目的草案；priority 1200 = 当前 active 波次之后的最低档延后项。
