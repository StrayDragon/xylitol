---
version: "alpha"
name: "transcript"
description: "Product conversation transcript — scrollback-native message stack."
tokens_from: "../DESIGN.md"
components:
  user-message:
    backgroundColor: "{colors.user-message-bg}"
    textColor: "{colors.on-surface}"
  user-prefix:
    textColor: "{colors.user}"
  assistant-body:
    textColor: "{colors.assistant}"
  system-line:
    textColor: "{colors.muted}"
---

# Transcript

> Token 根源：`{colors.*}` / `{components.*}` → [`../DESIGN.md`](../DESIGN.md)。

产品对话主内容区。实现：`app-tui-transcript`（c470）；包侧只提供 Text / Markdown / Diff 等积木。

## MUST

1. 全宽；历史经引擎滚入 scrollback，**MUST NOT** 截断历史冒充滚动。
2. 用户消息：配置档短前缀 + 正文；可选 `{colors.user-message-bg}` 全行底；**MUST NOT** 使用 `USER>` / `You` 等长标签。
3. 助手：正文直接出；**MUST NOT** 每段加 `ASSISTANT>`。
4. System / 错误：短 dim 或 `{colors.error}` 一行（见 [`errors.md`](./errors.md)）。
5. Markdown 规则见 [`markdown.md`](./markdown.md)；Diff 块见 [`diff-block.md`](./diff-block.md)。
6. thinking / tool 默认折叠摘要，展开规则与工具 bg 三态见 [`expandable.md`](./expandable.md)。

## Layout note

Transcript 在默认栈最上方；下方才是 status / editor / footer（见主 `DESIGN.md` Layout）。
