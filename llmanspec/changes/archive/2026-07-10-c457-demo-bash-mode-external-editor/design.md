# Design — c457-demo-bash-mode-external-editor

## Bash 边框

对齐 pi：bash 模式用 **前景** 强调边框（`colors.success`），不是 tool-*-bg。检测：编辑器文本 `trim_start` 后以 `!` 开头。

## Ctrl+G

与 Alt+G（glyphs）分离。demo stub：系统行 + 可选追加 `# $EDITOR stub`；**不**真 spawn，避免 harness/无 TTY 失败。
