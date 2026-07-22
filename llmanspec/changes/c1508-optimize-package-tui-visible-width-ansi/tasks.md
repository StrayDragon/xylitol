# Tasks: c1508-optimize-package-tui-visible-width-ansi

- [x] T1: `visible_width` — ANSI + ASCII printable 快路径（跳 ESC、不分配 cleaned `String`、不走 grapheme）
- [x] T2: 单测 — 着色 ASCII / OSC 链接 / 与慢路径混 CJK 宽度一致；既有 `utils_test` 全绿
- [x] T3: `just test-tui`（或至少 xylitol-tui utils + 相关包测）通过
- [x] T4: 勾选 tasks；proposal 记落地说明（仍无 MUST/SHALL 合约变更）
