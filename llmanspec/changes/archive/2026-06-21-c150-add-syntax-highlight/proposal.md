---
change_id: c150-add-syntax-highlight
depends_on: []
---

# c150-add-syntax-highlight: 添加语法高亮模块

## Why

pi 提供了 `syntax-highlight.ts` + `html.ts`（共约 207 行），使用 highlight.js 进行代码语法高亮：
- 语言自动检测
- 指定语言高亮
- 主题驱动的格式化器
- HTML 标签解析与渲染

xylitol 当前没有任何语法高亮能力。这对于未来 TUI 模式中的代码渲染至关重要。

## What Changes

在 `src/infra/syntax/` 下创建新模块，可选的 `infra-syntax` feature：

1. `mod.rs` — 公共 API（`highlight()`、`supports_language()`）
2. `theme.rs` — 主题定义（作用域到格式化器的映射）
3. `render.rs` — 语法高亮渲染输出

使用 `syntect` crate（Rust 原生语法高亮，基于 Sublime Text 的 .sublime-syntax）。

## Capabilities

- 新增 capability: `syntax-highlight`

## Impact

- 新增可选 feature `infra-syntax`
- 新增依赖 `syntect`
- 约 200-300 行新 Rust 代码
