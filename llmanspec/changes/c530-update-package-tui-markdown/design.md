# Design — c530-update-package-tui-markdown

## UX SSOT

行为与权衡以 [`src/app/tui/design/markdown.md`](../../../src/app/tui/design/markdown.md) 为准（token 效率 vs round-trip 标记白名单、表格方案 A）。

## 决策摘要

| 主题 | 选择 | 理由 |
|---|---|---|
| 标题 | 无 `#`；SGR 分级 | 复制省 token；层级靠色/下划线 |
| 链接 | `text (url)` | 终端多半不能内联跳转 |
| 代码块 | 无 fence | 装饰占 token；高亮用 SGR |
| 行内标记 | 保留 `` ` `` / `**` / `*` / `~~` | 用户要求可再解析 |
| 表格 | 空格对齐（A） | 无盒线、无装饰 `\|` |
| 引用 | 无 `│` | 同「少装饰」 |

## 主题 API

- 现有 `MarkdownTheme` 闭包保留；标题分级可经 `heading` 或按 level 分支（实现最小改动，可在 theme 内区分 H1–H6）。
- `code_block_border` / `quote_border`：默认 identity 空或不再调用装饰串；**不得**再默认输出 `` ``` `` / `│ `。
- `highlight_code`：仍可选；包默认不绑 syntect。

## 测试策略

- `#[cfg(test)]`：可见字符断言（剥 ANSI 后）覆盖标题无 `#`、链接形态、无 fence、无盒线、行内标记、引用无竖线、表空格对齐。
- `agent_demo`：seed 全语法 showcase + 流式代码块（源仍用 fence 供 syntect；显示无围栏）；`agent_demo_test` 覆盖。
- 不强制新 BDD feature（包级单测足够）。

## 非目标

- 产品面 theme token 映射实现（仍属 `app-tui-*` / 开闸后）
- playground 同步（轨 P 文档可选，非本 change 阻塞）
