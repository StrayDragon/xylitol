---
change_id: c630-add-app-tui-models-picker
title: "产品 /models：fuzzy 模型列表替换 editor 槽"
status: purpose-draft
priority: 630
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c630-add-app-tui-models-picker

## Why

`/model` 极简切换难用。用户需要 **`/models` + fuzzy 列表** 看得见、搜得到地换模型；配置仍走 YAML，本 change **不**提供运行时改配置。

## Purpose

斜杠 `/models` 打开替换 editor 槽的可选列表；fuzzy 过滤；Enter → `SetModel`；Esc 取消。移除产品主路径上的 `/model` cycle。

## What Changes（实现时）

1. `commands` / 补全：`/models`；停推 `/model`。
2. UI：SelectList（或等价）换 editor 槽；形状遵循 DESIGN。
3. `dispatch`：`GetAvailableModels` → 选定 `SetModel`；更新 footer model。
4. harness：过滤 / 选定 / Esc / idle-only。

## Capabilities

- `app-tui-commands`（modify：`/models`；retire `/model` 主路径）
- `app-tui-input`（modify：槽机打开 models）
- 可选 `app-tui-chrome`（footer 刷新）

## Design SSOT（MUST 遵循）

- [`src/app/tui/design/models-picker.md`](../../../src/app/tui/design/models-picker.md)
- [`src/app/tui/design/keybindings.md`](../../../src/app/tui/design/keybindings.md)
- playground 槽 `models`（`?slot=models`）

## Impact

- `src/app/tui/{commands,host,layout,effects}`；`app/core/dispatch`（已有 Command）
- **不**改 YAML schema；**不** Settings 槽

## Out of scope

- 模型市场 / OAuth；busy 中换模（默认 idle-only，除非 design 修订）

## Ethics

- risk_level: low
- prohibited_actions: 不引入运行时写配置；不回退为无列表 `/model` cycle
- required_evidence: harness 覆盖选定与 Esc；playground 形状对照

## Depends

- **c625**（设计闸）
