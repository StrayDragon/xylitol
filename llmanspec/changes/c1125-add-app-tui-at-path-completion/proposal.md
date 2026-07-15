---
change_id: c1125-add-app-tui-at-path-completion
title: "产品 TUI：@ 文件模糊引用（接线 AtPathSource）"
status: full
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

产品 editor 注册 `AtPathSource`（进程 cwd 为根，可测覆盖）；用户输入 `@` 模糊选文件并插入路径引用；提交语义为**路径文本**（不在本变更自动 read 文件内容）。

## What Changes

- `install_completion_sources` 加入 `AtPathSource`
- harness：`@` 弹出与 Tab 插入
- 参考 `agent_demo`；不复制业务到包内
- delta：`app-tui-input`（add ati33）

## Capabilities

- `app-tui-input`（modify/add）

## Out of scope

- `$skill`（c1130）
- 图片拖放
- 提交时自动读入文件内容
- 改包 `AtPathSource` 算法（除非发现产品接线 bug）

## Ethics

- risk_level: low
- prohibited_actions: 未信任扫描项目外敏感路径为默认（默认 cwd）
- required_evidence: harness 补全测
- escalation_policy: 提交时是否自动 read 文件需另开 change

## Depends

- []
