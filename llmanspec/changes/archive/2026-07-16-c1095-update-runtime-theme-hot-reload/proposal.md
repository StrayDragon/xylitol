---
change_id: c1095-update-runtime-theme-hot-reload
title: "Themes 热重载：磁盘主题 → 运行时 Palette/产品 theme"
status: draft
priority: 1095
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: runtime
ethics:
  risk_level: low
  prohibited_actions:
    - 重载清空会话 / transcript
    - 破坏 DESIGN token SSOT 双轨色板（dark/light 内建）
  required_evidence:
    - 坏主题名保留旧 Palette
    - 成功 apply light 后 UI 使用 light token（可测）
  escalation_policy: 与 DESIGN.md / sync-tui-tokens 冲突时升级确认
---

# c1095-update-runtime-theme-hot-reload

## Why

资源发现已能 list theme **文件名**；包侧有 `Palette::dark/light`；产品写死 `product_dark()`。`/reload`（c1120）与 `/theme`（c1115）需要热重载缝。

## Purpose

1. 发现磁盘主题名（经 app/core + Trust，对齐 bootstrap）。
2. 按名解析：**内建** `dark` / `light` → `Palette`；未知名失败并**保留旧主题**。
3. `LayoutTheme` / `UiRoot` / `HostSession` 可 apply；失败有诊断。
4. 复用 c1100 的 `XyReloadable` 于 loader（主题缓存随 loader reload 刷新）。

## What Changes

- `LayoutTheme::from_palette` / `product_light`
- `UiRoot::apply_layout_theme`（重建依赖 theme 的子组件）
- `app/core`：`discovered_theme_names`（Trust 闸）
- `app/tui`：`resolve_builtin_palette` + `HostSession::reload_themes`
- delta：`app-tui-chrome` · `runtime-resource-discovery`（轻量）

## Capabilities

- `app-tui-chrome`（modify）
- `runtime-resource-discovery`（modify）

## Out of scope

- `/theme` slash UI（c1115）
- 完整 pi 风格自定义 JSON 色板解析（后续；本波仅 dark/light 内建）
- 默认开启 theme auto（DESIGN 仍固定暗色 MVP 默认）

## Impact

- 解锁 c1115 / c1120 主题半边
- 与 c1100 独立；共享 loader `XyReloadable`
