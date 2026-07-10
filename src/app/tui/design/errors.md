---
version: "alpha"
name: "errors"
description: "Short error lines — prefer one-line error color over modal walls."
tokens_from: "../DESIGN.md"
components:
  error-line:
    textColor: "{colors.error}"
  warning-line:
    textColor: "{colors.warning}"
---

# Errors

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## MUST

1. 错误优先 **一行** `{colors.error}` 短文；需要详情时再展开或写日志。
2. **MUST NOT** 用多行红框墙或居中大弹窗挡 scrollback（确认类短交互除外，见 [`overlay.md`](./overlay.md)）。
3. 终端 restore / 致命渲染失败：退出前恢复 TTY（host / terminal_guard）；用户可见提示保持短。
