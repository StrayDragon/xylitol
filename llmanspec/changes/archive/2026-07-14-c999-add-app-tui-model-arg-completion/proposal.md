---
change_id: c999-add-app-tui-model-arg-completion
title: "产品 /model 参数内联补全（SlashArgCompletionSource）"
status: full
priority: 999
depends_on:
  - c630-add-app-tui-models-picker
author: agent
track: A
wave: deferred
---

# c999-add-app-tui-model-arg-completion

## Why

c630 落地无参 `/model` → editor 槽 SelectList；包侧已提供 [`SlashArgCompletionSource`](../../../packages/xylitol-tui/src/completion.rs)。产品仍缺：键入 `/model <前缀>` 时的内联补全。

## Purpose

产品 Editor 注册包 `SlashArgCompletionSource`（**默认无 bare**）+ `SlashCommandSource`；`/model `（有空格）自动弹出模型 id 补全；选定写入 editor，再经既有 `/model <id>` / `SetModel` 生效。无参 `/model` Enter 仍归 c630 槽。

## What Changes

1. 产品 `UiRoot`：`set_completion_sources` — `SlashArgCompletionSource::new("model", catalog).with_id("model-id")`（**不** `with_bare_command`）在前或后均可（默认 probe 互斥：有空格才 arg）；`SlashCommandSource` 含 `/exit` `/model`。
2. catalog：`(id, provider)` 来自 `Driver::available_models`；host 启动或 OpenModels 拉取后 `set_model_arg_catalog` 刷新。
3. **MUST NOT** 在 `src/app/tui` 再实现补全 popup；**MUST NOT** 产品 bare `/model` 抢 c630 槽（与 demo `with_bare_command(true)` 相反）。
4. harness：`/model dee` 出项；Tab 写入 id；Esc 关 popup 不改模型；无参 `/model` Enter 仍 OpenModels。

## Capabilities

- `app-tui-input` / `app-tui-commands`（modify）
- 包能力已落地（本 change **不**改 `packages/xylitol-tui`，除非发现 blocker）

## Design SSOT

- 本 change [`design.md`](./design.md)
- [`models-picker.md`](../../../src/app/tui/design/models-picker.md)

## Out of scope

- 重做 c630 槽；Ctrl+Shift+M；`/models`；写 YAML；产品 bare 开 catalog

## Ethics

- risk_level: low
- prohibited_actions: app 面复制 Editor 补全引擎；`with_bare_command(true)` 导致无参无法开槽
- required_evidence: harness 参数补全 + 无参开槽回归
- escalation_policy: 若必须改包 API，STOP 并 delta package-tui-*

## Depends

- **c630**（已归档）
