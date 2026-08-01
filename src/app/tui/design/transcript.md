---
version: "alpha"
name: "transcript"
description: "Live turn output into scrollback — not a Codex-style transcript browser."
tokens_from: "../DESIGN.md"
components:
  user-prefix:
    textColor: "{colors.user}"
  assistant-body:
    textColor: "{colors.assistant}"
  system-line:
    textColor: "{colors.muted}"
---

# Transcript（live 输出，非浏览面）

> Token 根源：`{colors.*}` / `{components.*}` → [`../DESIGN.md`](../DESIGN.md)。

**不是** Codex 式 transcript view。历史回看 / 分支 travel / fork 走 [`session-tree.md`](./session-tree.md)（双 Esc）。

本文件只约束：**当前轮**写入终端 scrollback 的呈现（若/当 bridge 接线 live 行时）。`app-tui-transcript` 语义为 live scrollback；**不得**按「专用 TranscriptView 组件树」实现。

## MUST（若有 live 行）

1. 全宽写入引擎 scrollback；**MUST NOT** 截断历史冒充滚动。
2. 用户：短前缀 + 正文；**默认无** `{colors.user-message-bg}` 全行淡底、**无** status 轨；**MUST NOT** `USER>` / `You` 长标签。
3. 助手：正文直接出（flush）；**MUST NOT** 每段 `ASSISTANT>`。
4. System / 错误：短 dim 或 `{colors.error}`（见 [`errors.md`](./errors.md)）。
5. Markdown / Diff 细则见 [`markdown.md`](./markdown.md)、[`diff-block.md`](./diff-block.md)——作为**行级积木**，不是独立浏览面。
6. thinking / tool 折叠：thinking **flush**（同正文，无轨）；tool/bash/diff **rail**——见 [`expandable.md`](./expandable.md)。
7. 相邻块之间 **≥1** 行 untinted 空行。

## MUST NOT

1. **MUST NOT** 实现 Codex 风格的可导航 transcript 浏览器 / 专用 `TranscriptView` 作为主 UX。
2. **MUST NOT** 用 transcript 栈替代会话树完成 branch travel。
3. **MUST NOT** 默认整行铺 `tool-*-bg` / `user-message-bg` 洗底（成败用左边轨）。

## Layout note

默认栈仍可有「上方内容区」占位；选择器打开时替换 **editor 槽**。导航真相源是会话树，不是内容区浏览器。
