---
change_id: c1120-add-app-tui-reload-slash
title: "产品 slash：/reload 热重载资源（不改历史）"
status: purpose-draft
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
---

# c1120-add-app-tui-reload-slash

## Why

pi `/reload` 重载 keybindings、extensions、skills、prompts、themes、context。xylitol 不做 extensions/prompt 产品面；需高质量：**可订阅通知**、**当前 session 外部/内部状态刷新**、**不影响历史对话记录**。

## Purpose

`/reload`（idle；busy 拒绝或排队——升格钉死）：

1. 依次（或并行安全）触发：keybindings · skills · mcp · themes · context 热重载
2. 汇总成功/失败诊断到 scrollback 系统块
3. 经订阅总线通知 host/UI
4. **MUST NOT** 改写已持久化 session 历史消息；**MUST NOT** 清空 transcript 已渲染历史块（可刷新「资源清单」类 live 区）

## What Changes（升格 full 时）

- slash + effects 编排各 reload API
- ReloadReport / 事件
- harness：历史条目不变 + 资源目录更新
- delta：`app-tui-commands` · `app-tui-host` · 各 foundation specs

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-host`（modify）

## Out of scope

- prompt templates reload（产品不做 prompt）
- pi Packages / extensions
- 自动文件监视（仅 slash 触发；watch 后置）

## Ethics

- risk_level: high
- prohibited_actions: 重载删改历史 jsonl；重载丢内置工具；静默 always-trust
- required_evidence: 历史哈希/条目数不变；MCP/skills 目录变化可测；失败部分成功报告
- escalation_policy: 任一 foundation 未升格 full 前本 change 不得 apply

## Depends

- c1080 · c1085 · c1090 · c1095 · c1100
