---
change_id: c1115-add-app-tui-theme-slash
title: "产品 slash：/theme 切换主题"
status: draft
priority: 1115
apply_band: P3-feature
depends_on: ["c1095-update-runtime-theme-hot-reload"]
author: agent
track: R
wave: slash-simple
domain: app-tui
ethics:
  risk_level: low
  prohibited_actions:
    - 默认开启 theme auto / OSC11 探测（仍属 demo-only）
    - 抄 demo Ctrl+P theme-toggle 为产品默认键位
    - 未知主题名静默当作成功或清空 transcript
  required_evidence:
    - harness：/theme light 后 palette/light 可测；坏名保留旧主题
    - busy 拒绝
    - 无参打开主题槽
  escalation_policy: 槽形态已钉 SelectList（对齐 /model）；勿扩 Settings 板
---

# c1115-add-app-tui-theme-slash

## Why

包侧 Palette/`/theme` 在 demo 有验证；c1095 已提供 `HostSession::reload_themes` + 内建 `dark`/`light`。产品 TUI 尚无 `/theme` slash。

## Purpose

产品 TUI **idle** 解析 `/theme`：

1. **无参**：打开替换 editor 槽的主题 SelectList（内建 `dark` / `light`）；选中后经 `reload_themes` 应用并关槽
2. **有参** `dark` | `light`：直接 `reload_themes`；成功系统块提示；失败保留旧主题并提示
3. **有参** `toggle` | `cycle`：在 dark↔light 间切换（相对当前 `theme_preference` 或 palette）
4. **busy** MUST 拒绝且 MUST NOT 改主题
5. SlashCommandSource MUST 列出 `theme`；空格后参数补全至少含 `dark`/`light`（可含 `toggle`）
6. MUST NOT 默认开启 theme auto；MUST NOT 引入 demo Ctrl+P 为产品默认

## What Changes

- `commands` / catalog / `SlashArgCompletionSource`：`/theme`
- `EditorSlot::Themes`（或等价）+ `UiRoot` mount/select 路径（对齐 models 槽，非 Plate/Settings/Choice 活板）
- `effects/slash`：busy 闸 + 无参开槽 / 有参 apply
- harness：light 成功、坏名、busy、catalog
- 同步 `design/theme-tokens.md`：产品允许**用户发起** `/theme`，仍禁止默认 auto
- delta：`app-tui-commands` · `app-tui-chrome`

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-chrome`（modify）

## Out of scope

- 自定义 JSON 色板文件解析（c1095 已钉后续）
- theme auto / OSC11 / COLORFGBG 产品默认
- Settings 板 / Ctrl+P plate theme-toggle
- Driver 缝（主题纯 host；经已有 `reload_themes`）

## Impact

- 会话内可在暗/亮色板间切换且 `/reload` 能保留 `theme_preference`
