# Tasks — c540-update-package-tui-diff-edges

- [x] 1. Delta `package-tui-diff`（ptd8–ptd9）通过 `llman sdd validate c540-update-package-tui-diff-edges --no-interactive`
- [ ] 2. 补 CJK 窄宽与 SBS 空半栏单测（优先扩既有 `diff.rs` 测试）
- [ ] 3. 为 edit-format 或 SBS 增加/更新 snapshot（或强 assert）
- [ ] 4. `cargo test -p xylitol-tui diff`；`llman sdd validate … --strict`
