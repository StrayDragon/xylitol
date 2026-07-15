---
change_id: c1110-add-app-tui-history-copy-last
title: "产品 slash：/history-copy-last 复制最后 assistant 消息"
status: draft
priority: 1110
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: slash-simple
domain: app-tui
ethics:
  risk_level: low
  prohibited_actions:
    - 经本命令把剪贴板内容发到远程分享通道
    - 从 app/tui 直接 reach infra::clipboard
  required_evidence:
    - 有/无 assistant 路径 harness
    - 失败系统提示
  escalation_policy: busy 已钉「允许只读复制」；Ctrl+X 键位本 change 不做
---

# c1110-add-app-tui-history-copy-last

## Why

PLAN 选中 `/history-copy-last`（pi `/copy`）。clipboard infra 已存在；产品 TUI 未接线。

## Purpose

产品 TUI **idle 与 busy 均可**解析无参 `/history-copy-last`：

1. 取当前 transcript 中最近一条已提交的 assistant 文本（`UiEntry::Assistant`；不含 streaming 半句，除非 design 另钉）
2. 经 Driver 缝调用 `copy_to_clipboard`
3. 成功：系统块短提示（可含字符数）；无消息 / 失败：系统块错误提示
4. SlashCommandSource MUST 列出该命令；带参 MUST usage

## What Changes

- `commands` + product catalog
- `Driver::copy_text_to_clipboard`（InProcess → infra；Scripted 可观测）
- `effects/slash`：选文 + copy + 提示
- harness：有/无 assistant；stub clipboard 调用次数
- delta：`app-tui-commands` · `app-tui-host`（或仅 commands+host）；infra-clipboard 仅引用不改 MUST（除非需产品缝说明）

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-host`（modify）

## Out of scope

- gist `/share`
- Ctrl+X / 键位绑定（可后置）
- 树内复制选中节点
- 复制 thinking / tool 块（仅 assistant 正文）

## Impact

- 会话内一键复制最后回复到系统剪贴板
