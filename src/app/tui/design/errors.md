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
4. **进入 raw mode 前**（CLI `preflight`）：非 TTY、无选中 model、无法读取终端尺寸 → **直接 CLI `Error:` 退出**，MUST NOT 先改终端状态再失败。过小终端允许进入（in-TUI「请放大」）；API 连通性探测后置、默认不挡启动。
