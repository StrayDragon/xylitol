# Tasks — c530-update-package-tui-markdown

- [x] 1. 确认 delta `package-tui-markdown`（ptm0–ptm8）通过 `llman sdd validate c530-update-package-tui-markdown --no-interactive`（`--strict` 待全部 task 勾完）
- [x] 2. 改 `packages/xylitol-tui/src/components/markdown.rs`：标题无 `#`、链接 `text (url)`、代码块无 fence、引用无 `│`、表格空格对齐、行内保留标记
- [x] 3. 更新/新增包内单测（剥 ANSI 可见字符合约）；修正依赖旧输出的断言
- [x] 4. 若 `agent_demo` 断言依赖 fence / `#` 前缀：检查后无需修正（高亮测只断言 ANSI / 正文）
- [x] 5. 同步 DESIGN：`design/markdown.md` 实现状态、`DESIGN.md` Don'ts、`theme-tokens.md`、playground Markdown 槽
- [x] 6. 验证：`cargo test -p xylitol-tui`；`llman sdd validate c530-update-package-tui-markdown --strict --no-interactive`
