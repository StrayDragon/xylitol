---
change_id: c530-update-package-tui-markdown
title: "package-tui-markdown：token 友好复制渲染（对齐 design/markdown.md）"
status: full
priority: 530
depends_on: []
author: agent
track: P
---

# c530-update-package-tui-markdown

## Why

产品 DESIGN 已决议 Markdown **省 token、可复制、信息不丢**：层级靠 SGR（色/粗/下划线），不用 `#` 前缀与盒线装饰；链接必须 `text (url)`；代码块无 fence；表格空格对齐；行内 `` ` `` / `**` / `*` / `~~` 保留以便 round-trip。

现状 `packages/xylitol-tui` Markdown 仍输出 `#`、`` ``` ``、引用 `│`、盒线表，且链接只吐 URL——与 [`src/app/tui/design/markdown.md`](../../../src/app/tui/design/markdown.md) 冲突。

## Purpose

新建 capability `package-tui-markdown`，把包内 `Markdown` 组件行为收敛到 DESIGN MUST；用单测/harness 锁住复制可见字符合约。

## What Changes

1. 新 capability **`package-tui-markdown`**（名词域：包 Markdown 渲染合约）。
2. 标题：色组闭包 + bold/underline；**禁止** `#`/`##` 可见前缀。
3. 链接/图片：`text (url)` / `alt (url)`；禁止只 OSC 或丢 URL。
4. 代码块：无 fence / 语言条 / 行号墙；高亮仍经 `highlight_code` 回调（c452 约定不变）。
5. 行内：可见保留 `` ` ``、`**`、`*`、`~~`。
6. 引用：无 `│`；仅 dim/italic。
7. 表格：方案 A 空格对齐 + 表头 underline；无盒线、无装饰 `|`。
8. 列表保留 `- `/`1. `；HR 短线；fg/bg 分相不变。
9. 同步单测（及必要 snapshot）；**不**改产品 `src/app/tui` host 接线。

## Capabilities

- `package-tui-markdown`（新建）

## Impact

- `packages/xylitol-tui/src/components/markdown.rs` 及包内测试
- 可选：`agent_demo` 仅当现有断言依赖旧 fence/`#` 时最小修正
- UX SSOT 已在 `src/app/tui/design/markdown.md`（本变更实现侧对齐，不重开产品面）

## Out of scope

- 产品 host / bridge / theme 注入接线（开闸后轨 B）
- 表格改回 GFM `|`（方案 B；需另变更）
- 把 syntect 打进默认依赖（仍 c452 / optional `highlight`）
- playground HTML（人类示意；非本包运行时）

## Ethics

- risk_level: low
- prohibited_actions: 不在冻结期堆 `src/app/tui` 产品视觉接线
- required_evidence: `cargo test -p xylitol-tui` 相关用例；`llman sdd validate` 通过
