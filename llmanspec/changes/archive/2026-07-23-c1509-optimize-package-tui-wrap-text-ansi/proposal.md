---
change_id: c1509-optimize-package-tui-wrap-text-ansi
title: package-tui：wrap_text_with_ansi ANSI+ASCII 快路径
status: applied
# archived: 2026-07-23 (docs-only; quick landed on main)
priority: 1509
depends_on:
  - c1508-optimize-package-tui-visible-width-ansi
author: agent
---

# c1509-optimize-package-tui-wrap-text-ansi

> **Archived** `llmanspec/changes/archive/2026-07-23-c1509-optimize-package-tui-wrap-text-ansi/`
> Promoted → applied（无 MUST/SHALL；quick 落地）。验收：`utils_test` wrap + xylitol-tui 包测。

## Why

`post-c1508` C-stream samply：`visible_width`/`strip_ansi` 已降；流式 markdown 热点转为 **`wrap_text_with_ansi` ~19%**（经 `scrollback` → `markdown`）。

现实现 `split_into_tokens_with_ansi` 对**纯 ASCII / ANSI+ASCII** 也走 `UnicodeSegmentation::graphemes`，流式着色正文白白付 grapheme 税。

## What Changes

1. `split_into_tokens_with_ansi`：ANSI+ASCII（可含空格）走字节分词，**不** grapheme；tab/CJK/emoji 仍慢路径
2. `wrap_text_with_ansi`：`prefix` 为空时避免 `format!` 拼行
3. 单测：ANSI 着色 ASCII 换行结果与慢路径一致；CJK 行为不变
4. 验收：`cargo test -p xylitol-tui --test utils_test`；可选 `profile-suite C`

## Out of scope

- c1505 viewport；改 markdown AST；去掉宽度不变量

## Status

**applied** — `split_into_tokens_ansi_ascii` + 无 prefix 时免 `format!`。

## Ethics

- risk_level: low
- prohibited_actions: 改变换行/ANSI 继承可观测结果；把 CJK 误判进 ASCII 快路径
