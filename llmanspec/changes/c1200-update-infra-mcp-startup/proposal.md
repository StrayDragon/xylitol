---
change_id: c1200-update-infra-mcp-startup
title: "MCP 启动体验：延迟/策略待议"
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

# c1200-update-infra-mcp-startup

## Why

启用 MCP 后，启动或首次进入 print·TUI 的首交互延迟明显上升；提示词/工具目录注入时机也可能过早或过重。需要专门一轮讨论**启动策略**（不预设唯一方案）。

## Purpose（**暂缓 / deferred · 最低优先级**）

本变更 **P9 暂缓**：不挡现有产品 slash 波次。升格前须先收敛方案（见下方候选），再写 MUST。

**问题空间（升格时钉死，勿锁死实现）**：

- 首交互时间（TTI）与 MCP 连接成本的权衡
- 工具目录 / MCP 摘要进 prompt 的注入点与形状
- 与 `/reload`、Trust、失败诊断的兼容

**候选方向（非合约，可替换）**：后台/懒连接、并行发现、分阶段注入、超时降级、显式「MCP ready」信号等——升格时择一或组合。

## What Changes（升格 full 时）

- 依选定方案改 bootstrap / composition / prompt 缝
- delta：`infra-mcp` 及受影响的 `app-core` / `agent-prompt`
- 可观测：configured / connecting / connected（或等价）

## Capabilities

- `infra-mcp`（modify）
- 视方案：`app-core` / `agent-prompt`

## Out of scope

- 本波实现（deferred）
- 改 MCP 线协议 / 新 transport
- 替代 c1135 header 可见性

## Ethics

- risk_level: medium
- prohibited_actions: 静默丢弃已配置 MCP；把密钥写进 prompt
- required_evidence: 有/无 MCP 路径可测；选定方案有基线对比
- escalation_policy: 方案歧义时先 explore / 问用户再升格 full

## Depends

- c1080 · c1120（归档名以仓库为准）

## Notes

- 2026-07-16：自 `c1200-update-infra-mcp-lazy-bootstrap` 泛化重命名，避免过早锁定「懒加载」单一解。
