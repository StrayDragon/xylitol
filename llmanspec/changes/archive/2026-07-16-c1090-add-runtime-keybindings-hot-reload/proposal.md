---
change_id: c1090-add-runtime-keybindings-hot-reload
title: "Keybindings 热重载缝：app.* 目录 + 磁盘重载 + 产品改匹配"
status: full
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

`/reload`（c1120）要对齐 pi「重载 keybindings」。包已有 `KeybindingsManager`，但（1）产品大半键位硬编码 `matches_key_event("ctrl+t")`，热重载无法改产品行为；（2）无磁盘 `keybindings.json` 加载缝；（3）user config 类型为 `&'static str`，无法承接 JSON。

## Purpose

把 keybindings 做成可维护、可热重载资源：产品动作以 `app.*` id 注册；匹配一律走 Manager；从 user 目录读 `keybindings.json`；提供 `reload`（失败保留旧绑定并诊断）；通知订阅者刷新（如 TreeHelp）。不碰会话历史。

## What Changes

- 包：`KeybindingsConfig` 支持拥有型 `String` 覆盖；可合并额外 definitions
- 产品：`app.*` 默认目录（树 filter / interrupt / follow-up / Ctrl+G / resume 等已接线键）
- 产品：slot_input / input_policy / session_resume / host listener 改走 id 匹配
- 加载：`~/.xylitol/keybindings.json`（或 agent_dir 等价）；`reload_keybindings` API
- 失败保留旧绑定；可选 EventBus / 回调通知
- delta：`package-tui-keybindings`（新）· `app-tui-input`（ati35）

## Capabilities

- `package-tui-keybindings`（add）
- `app-tui-input`（add ati35）

## Out of scope

- `/reload` 总控（c1120）
- 改默认和弦语义（仅把硬编码迁到同默认的 id）
- 全量 pi `app.*`（未接线动作可不注册或 defaultKeys 空）
- 原子 marker / 主题 / context 重载（c1095/c1100）
- agent_demo 全量改 id（MAY 跟进；产品优先）

## Ethics

- risk_level: low
- prohibited_actions: 重载清空 transcript；坏文件覆盖为坏绑定
- required_evidence: 坏 JSON 保留旧绑定；成功后改 id 映射生效测
- escalation_policy: —

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
