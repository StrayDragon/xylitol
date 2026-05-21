---
depends_on: [c30-add-print-mode, c80-add-tui]
---

# c90-update-markdown-rendering

## Why

目前 xylitol 的终端输出（Print/TUI）对 Markdown 的着色渲染存在一致性与兼容性风险：

- 行折叠/换行规则在不同终端宽度与宽字符（CJK/emoji）场景下可能不稳定
- 语法高亮、链接样式、代码块、表格等 GFM 扩展能力缺少统一的“可预期行为”规范
- 渲染实现与依赖选择缺少明确的评估基准（正确性/性能/依赖许可/可维护性）

本变更的目标是：为“终端可读的 Markdown 渲染”建立明确能力边界与验收标准，并选择一个兼容性强、许可友好、可长期维护的实现方案。

## What Changes

1. 新增 capability `markdown-rendering`：定义终端 Markdown 渲染的需求与验收场景（与 Print/TUI 共享）。
2. 建立评估基准：用快照测试覆盖常见 Markdown 元素与边界条件（宽字符、长单词、嵌套样式、代码块高亮、链接样式等）。
3. 调研并对比候选方案（示例）：
   - 继续使用 `pulldown-cmark` 解析 + 自研渲染（保持最小依赖，成本较高）
   - 引入成熟的 ratatui markdown 渲染库（要求：非 GPL，能与 `syntect` 配合）
4. 选定并落地一个默认方案，并在必要时保留 `raw output` 作为一致性兜底。

## Capabilities

- `markdown-rendering`: 终端 Markdown 渲染（Print/TUI 共享）

## Impact

- 输出样式可能产生可见变化（需要快照测试锁定行为）
- 可能引入/替换依赖（必须审查许可与体积/性能）
- 需要与现有主题/配色策略（syntect theme）衔接，避免“强制蓝色+下划线”等不可控样式
