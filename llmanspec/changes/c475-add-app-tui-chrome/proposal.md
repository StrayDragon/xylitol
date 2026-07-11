---
change_id: c475-add-app-tui-chrome
title: "app-tui-chrome：theme、glyph、status、footer"
status: purpose-draft
priority: 475
depends_on: ["c460-add-app-tui-host", "c449-split-app-tui-design-docs"]
author: agent
track: B
---

# c475-add-app-tui-chrome

> **status: purpose-draft**（升格前可改；apply 顺序建议在 c465 后或紧随）

## Purpose

语义 token→闭包；glyph unicode/ascii；idle 无 status；busy 一行；footer=`cwd · model`（branch/context% future）。
包侧已有 `Palette` Dark/Light 工厂（c570）——产品 MVP **固定暗色**注入即可。

## Out of scope

- 产品默认开启自动亮暗 / `/theme` 市场（demo 已有 `/theme` + 可选 `THEME_AUTO`；产品勿默认开 auto）
- ChoicePrompt / Ask 工具接线（包 c565 已有；产品工具面后置）
