---
change_id: c999-add-app-tui-model-arg-completion
title: "后置：/model 参数自动补全（CompletionSource）"
status: purpose-draft
priority: 999
depends_on:
  - c630-add-app-tui-models-picker
author: agent
track: A
wave: deferred
---

# c999-add-app-tui-model-arg-completion

## Why

c630 落地无参 `/model` → **editor 槽 SelectList** 后，仍缺 pi/截图式体验：在 editor 里继续键入 `/model <前缀>` 时弹出 **内联自动补全**（副列 provider、fuzzy、↑↓/Tab）。主体列表与补全解耦，避免主线膨胀。

## Purpose

在 **不重做 popup 引擎** 的前提下，用 `packages/xylitol-tui` 已有 [`CompletionSource`](../../../packages/xylitol-tui/src/completion.rs) 注册表，为 `/model ` 参数提供模型 id 补全；选定后写入 editor（或等价应用），再经既有 `/model <id>` / `SetModel` 路径生效。

## What Changes（实现时）

1. 产品面实现并注册 `ModelIdCompletionSource`（或通用 `SlashArgCompletionSource` 仅服务 model）：
   - `probe`：光标前匹配 `/model` + 空格 + 可选前缀（**有空格**才接管；无参 `/model` 仍归 c630 槽）。
   - `suggestions`：`GetAvailableModels`（或 host 缓存）→ fuzzy；`label`=id，`description`=provider/短标签。
   - `apply`：把模型 id 写回当前行（对齐包 `SlashCommandSource::apply` 习惯）。
2. 接线：产品 Editor / layout 在装配时 `set_completion_sources`（与 `/` 命令名 Source、`@` 等并存；registry 顺序：slash 命令名优先于 model 参数，或 probe 互斥）。
3. **MUST NOT** 在 `src/app/tui` 再实现一套补全 popup / SelectList 生命周期。
4. harness：`/model dee` 出项；Tab/Enter 写入；Esc 关 popup 不改模型；与 c630 无参开槽不打架。

## Package vs product

| 层 | 做 |
|---|---|
| `xylitol-tui` | 已有 popup；本 change **默认不改包**。若多命令要复用 slash-arg，MAY 小幅增强 `SlashCommandSource` / 抽出 helper（开闭），但非 blocker。 |
| `src/app/tui` | Source 实现 + 模型列表数据 + 注册 |

## Capabilities（实现时）

- `app-tui-input` / `app-tui-commands`（modify：参数补全）
- 可选 `package-tui-*`（仅当抽出通用 slash-arg）

## Design / UX

- 形状对齐包 Autocomplete：主列 id、副列 description、`(n/m)` 由包列表组件负责。
- 与 c630 槽：**互补**——无参 Enter → 槽；键入参数 → 内联补全。
- SSOT 指针：c630 / [`models-picker.md`](../../../src/app/tui/design/models-picker.md)；补全细节实现时补一行 MUST 即可。

## Out of scope

- 重做 c630 槽机；Ctrl+Shift+M；`/models` 别名；运行时写 YAML；模型市场。

## Ethics

- risk_level: low
- prohibited_actions: 在 app 面复制 Editor 补全引擎；抢无参 `/model` 的 probe 导致无法开槽
- required_evidence: harness 参数补全 + 与 c630 无参开槽回归
- escalation_policy: 若必须改包公开 API，先 delta `package-tui-*` 再 apply

## Depends

- **硬依赖 c630**（主路径 `/model` 槽 + `/model <id>` 直选已归档）
- 软依赖：包 `CompletionSource` 已存在（无需新 package change，除非选「通用 slash-arg」增强）
