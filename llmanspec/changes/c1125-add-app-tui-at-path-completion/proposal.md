---
change_id: c1125-add-app-tui-at-path-completion
title: "产品 TUI：@ 文件模糊引用（接线 AtPathSource）"
status: purpose-draft
priority: 1125
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: editor-ux
domain: app-tui
---

# c1125-add-app-tui-at-path-completion

## Why

`packages/xylitol-tui` 已有 `AtPathSource`；`agent_demo` 已注册。产品 `UiRoot::install_completion_sources` 仅挂 slash / `/model`，缺 `@`。

## Purpose

产品 editor 注册 `AtPathSource`（cwd 为根）；用户输入 `@` 模糊选文件并插入路径；提交后的附件语义（纯文本路径 vs 读入内容）升格时钉——默认对齐 demo/pi 的路径引用，读入内容可后置。

## What Changes（升格 full 时）

- `install_completion_sources` 加入 `AtPathSource`
- harness：`@` 弹出与插入
- 参考 `agent_demo` 接线，不复制业务到包内
- delta：`app-tui-input` · `package-tui-autocomplete`

## Capabilities

- `app-tui-input`（modify）
- `package-tui-autocomplete`（引用/小改）

## Out of scope

- `$skill`（c1130）
- 图片拖放

## Ethics

- risk_level: low
- prohibited_actions: 未信任扫描项目外敏感路径为默认（cwd 根）
- required_evidence: harness 补全测
- escalation_policy: 提交时是否自动 read 文件需产品确认

## Depends

- []
