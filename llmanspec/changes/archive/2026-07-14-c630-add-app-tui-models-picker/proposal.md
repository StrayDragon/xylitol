---
change_id: c630-add-app-tui-models-picker
title: "产品 /model：fuzzy 模型列表替换 editor 槽（对齐 pi）"
status: full
priority: 630
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c630-add-app-tui-models-picker

## Why

当前 `/model` 极简 cycle 难用。对齐 **pi**：同一命令名 **`/model`**，但行为改为 **看得见、搜得到的 fuzzy 列表**（替换 editor 槽），而不是静默轮换。配置仍走 YAML；本 change **不**提供运行时改配置。

## Purpose

斜杠 **`/model`**（无参）打开替换 editor 槽的可选列表；fuzzy 过滤；Enter → `SetModel`；Esc 取消。可选 **`/model <id>`** 仍直选（不经列表）。**不**引入 `/models` 别名。

## What Changes（实现时）

1. `commands`：无参 `/model` → 打开 picker（停掉 `CycleModel` 主路径）；有参 `/model <id>` → 既有 `SetModel`。
2. UI：SelectList（或等价）换 editor 槽；形状遵循 DESIGN。
3. `dispatch`：打开时 `GetAvailableModels` → 选定 `SetModel`；更新 footer model。
4. 补全 / 提示只推 `/model`（及可选 id 参数），**不**推 `/models`。
5. harness：打开 / 过滤 / 选定 / Esc / idle-only；断言无参不再 cycle。

## Capabilities

- `app-tui-commands` / `app-tui-input`（modify：`/model` → picker；retire cycle）
- 可选 `app-tui-chrome`（footer 刷新）

## Design SSOT（MUST 遵循）

- [`src/app/tui/design/models-picker.md`](../../../src/app/tui/design/models-picker.md)
- [`src/app/tui/design/keybindings.md`](../../../src/app/tui/design/keybindings.md)
- playground 槽 `models`（`?slot=models`）

## Impact

- `src/app/tui/{commands,host,layout,effects}`；`app/core/dispatch`（已有 Command）
- **不**改 YAML schema；**不** Settings 槽

## Out of scope

- `/models` 别名；模型市场 / OAuth；busy 中换模（默认 idle-only，除非 design 修订）
- pi 的 `Ctrl+Shift+M` 快捷键（MAY 后续；本 change 只钉斜杠）
- **editor 内联 `/model <前缀>` 自动补全** → 后置 **c999-add-app-tui-model-arg-completion**（依赖本 change + 包 `CompletionSource`）

## Ethics

- risk_level: low
- prohibited_actions: 不引入运行时写配置；不保留无参 `/model` 静默 cycle；不新增 `/models` 主路径
- required_evidence: harness 覆盖选定与 Esc；playground 形状对照
- escalation_policy: 无

## Depends

- **c625**（设计闸）
