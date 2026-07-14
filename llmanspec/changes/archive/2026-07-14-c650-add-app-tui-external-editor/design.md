# Design — c650-add-app-tui-external-editor

## Decisions

| 主题 | 决议 |
|---|---|
| 键位 | Ctrl+G（既有）；overlay/非 Editor 槽时不抢（沿用 stub 门闸） |
| TTY 真路径 | `TUI::with_terminal_suspended` → 写 tempfile → spawn `$VISUAL`/`$EDITOR` → 成功则 `set_text` |
| 解析顺序 | 非空 `$VISUAL`，否则非空 `$EDITOR`；**皆无则 Error** |
| 默认编辑器 | **无**（严格）；**MUST NOT** nano/notepad 静默兜底（与 demo 可默认 nano **刻意分叉**） |
| harness / 非 TTY / 测试 | stub（系统提示 + 可选 `# $EDITOR stub`）；**永不** spawn |
| 失败展示 | **一律** `UiEntry::Error`（`error: …`）；保留 Editor 原文；不 panic |
| 失败集合 | 未配置；tempfile 写/读失败；spawn 失败；编辑器非零退出 |
| 包边界 | 包**只**提供 `with_terminal_suspended`；产品 tempfile/spawn **独立实现于** `src/app/tui/` |
| 与 demo 复用 | **不复用**：包可单独分发；demo 与产品各管各的 editor 辅助逻辑（形状可参考，禁止抽共享进包） |

## Error 文案（建议，实现可微调）

- 未配置：`set $VISUAL or $EDITOR to use external editor (Ctrl+G)`
- spawn/IO：`external editor failed: …`
- 非零退出：`external editor exited non-zero — keeping original text`

对齐 [`errors.md`](../../../src/app/tui/design/errors.md)：一行 error 色，不当墙。

## Env / 门闸（产品）

| 条件 | 路径 |
|---|---|
| harness / `cfg(test)` / 非 stdin TTY | stub |
| 可选强制 stub env（若实现，对齐 demo 名或产品专用） | stub |
| 交互 TTY 且 VISUAL/EDITOR 已配置 | 真路径 |
| 交互 TTY 但未配置 | `UiEntry::Error`（不 stub 伪装成功） |

## DESIGN 文档同步

- `bash-mode.md`：产品行改为「严格无默认编辑器 + Error 管线」；demo 行可保留 nano 默认。
- `keybindings.md`：Ctrl+G 注记与上一致。

## Non-goals

- `llman-sdd-quick` 增强包导出 editor helper
- 统一 demo/产品单一实现
- footer / token 用量（c655 已 pause）
