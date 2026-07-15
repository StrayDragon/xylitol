---
change_id: c1120-add-app-tui-reload-slash
title: "产品 slash：/reload 热重载资源（不改历史）"
status: draft
priority: 1120
apply_band: P3-feature
depends_on:
  - c1080-update-infra-mcp-client-product
  - c1085-update-agent-skills-runtime
  - c1090-add-runtime-keybindings-hot-reload
  - c1095-update-runtime-theme-hot-reload
  - c1100-update-runtime-context-hot-reload
author: agent
track: R
wave: reload
domain: app-tui
ethics:
  risk_level: high
  prohibited_actions:
    - 重载删改历史 jsonl
    - 重载丢内置工具
    - 静默 always-trust
  required_evidence:
    - 历史哈希/条目数不变
    - MCP/skills 目录变化可测
    - 失败部分成功报告
  escalation_policy: foundation 已归档；apply 失败则回滚 slash 接线不改 reload API
---

# c1120-add-app-tui-reload-slash

## Why

pi `/reload` 重载 keybindings、extensions、skills、prompts、themes、context。xylitol 不做 extensions/prompt 产品面；需高质量：**可订阅通知**、**当前 session 外部/内部状态刷新**、**不影响历史对话记录**。MCP 运行时（c1080）与 skills（c1085）已就绪，缺产品 slash 编排。

## Purpose

`/reload`（**idle only**；busy MUST 拒绝并提示，MUST NOT 排队改历史）：

1. 依次触发：keybindings · skills · mcp · themes · context 热重载（各 foundation API）
2. 汇总成功/失败诊断到 scrollback 系统块
3. 刷新 `$skill` 补全 catalog（及可选 theme 相关 UI 状态）
4. **MUST NOT** 改写已持久化 session 历史；**MUST NOT** 清空 transcript 已渲染历史块

## What Changes

- `commands` + slash catalog：`/reload`
- `effects` / host：编排 `reload_*` helpers + MCP session reload
- harness：历史条目不变 + 至少一类资源目录更新可测
- delta：`app-tui-commands` · `app-tui-host`

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-host`（modify）

## Out of scope

- prompt templates reload
- pi Packages / extensions
- 自动文件监视（仅 slash；watch 后置）
- c1135 startup header（仍 deferred）

## Impact

- 解锁产品侧 MCP/skills/theme/context 热更新路径；对齐 pi `/reload` 主语义子集
