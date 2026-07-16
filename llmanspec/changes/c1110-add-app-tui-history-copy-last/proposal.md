---
change_id: c1110-add-app-tui-history-copy-last
title: "产品 slash：/history-copy-last 复制最后 assistant 消息"
status: purpose-draft
priority: 1110
depends_on: []
author: agent
track: R
wave: slash-simple
domain: app-tui
---

# c1110-add-app-tui-history-copy-last

## Why

PLAN 选中 `/history-copy-last`（pi `/copy` / Ctrl+X）。clipboard infra 已存在；产品 TUI 未接线。

## Purpose

idle `/history-copy-last`：将最近一条可复制的 assistant 文本经 clipboard 写入；无消息 / 失败时系统提示；busy 策略升格时钉（默认拒绝或允许只读复制）。

## What Changes（升格 full 时）

- slash 解析 + effects → `copy_to_clipboard`
- 可选键位（不强制 Ctrl+X；可后置）
- harness（可 stub clipboard）
- delta：`app-tui-commands` · `infra-clipboard`

## Capabilities

- `app-tui-commands`（modify）
- `infra-clipboard`（modify 或引用）

## Out of scope

- gist `/share`
- 树内复制选中节点（可后置）

## Ethics

- risk_level: low
- prohibited_actions: 复制密钥明文到远程分享通道（本命令仅本地 clipboard）
- required_evidence: 有/无消息路径；失败提示
- escalation_policy: 无 clipboard feature 时是否编译排除需确认

## Depends

- []
