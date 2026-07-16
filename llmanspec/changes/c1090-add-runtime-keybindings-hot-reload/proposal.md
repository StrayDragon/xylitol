---
change_id: c1090-add-runtime-keybindings-hot-reload
title: "Keybindings 热重载缝：资源加载 + 订阅通知"
status: purpose-draft
priority: 1090
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: runtime
---

# c1090-add-runtime-keybindings-hot-reload

## Why

`/reload`（c1120）要对齐 pi「重载 keybindings」语义。产品 TUI 已有键位表，但缺统一「从磁盘再读 → 通知订阅者 → 不碰会话历史」的资源缝。

## Purpose

把 keybindings 提升为可热重载资源：加载、校验、替换当前会话绑定；经订阅/事件通知 UI；失败时保留旧绑定并报告。

## What Changes（升格 full 时）

- 资源路径约定（user/project，受 Trust 闸）
- `reload_keybindings`（或等价）API + 诊断
- 订阅通知（供 host 刷新 `KeybindingsManager`）
- delta：新或 modify `app-tui-input` / runtime 资源相关 spec

## Capabilities

- `app-tui-input`（modify）或新建 `runtime-keybindings`（升格时定）

## Out of scope

- 改默认和弦语义（仅热重载机制）
- `/reload` 总控（c1120）

## Ethics

- risk_level: low
- prohibited_actions: 重载时清空 transcript / session 历史
- required_evidence: 坏文件保留旧绑定；成功后新键生效测
- escalation_policy: 与包层 `KeybindingsManager` API 冲突时先改包再接线

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
