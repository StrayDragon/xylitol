---
change_id: c475-add-app-tui-chrome
title: "app-tui-chrome：theme、glyph、status、footer"
status: ready
priority: 475
depends_on: ["c460-add-app-tui-host", "c449-split-app-tui-design-docs", "c465-add-app-tui-bridge"]
author: agent
track: B
---

# c475-add-app-tui-chrome

## Why

c465 已把 EventStream 接到占位 UI，但 chrome 仍塞快捷键墙、把 Working 并进 footer，且未注入 `Palette` / glyph。需按 `DESIGN.md` 落地极简 chrome，否则产品面体感与设计 SSOT 持续分叉。

## What Changes

1. `theme` / `glyphs` 薄模块：`Palette::dark()` → 闭包主题；glyph 档 env 切换。
2. `UiRoot`：独立 status 槽（idle 0 行）；footer=`cwd · model`（+ 可选 `q:`）；去键墙。
3. host / `run` 注入 cwd + model；scrollback 前缀走 glyph。
4. 单测覆盖 idle/busy/footer/ascii。

## Capabilities

- `app-tui-chrome`

## Impact

- `src/app/tui/**`（ui_root / host / theme / glyphs / tests）

## Out of scope

- 产品默认 theme auto / `/theme` 市场
- ChoicePrompt / Ask 工具接线
- c480 slash / abort；c491 活树
- Loader/braille spinner 精修（busy 短文案即可）
