# Tasks — c1005-update-app-tui-session-slash-rename

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1005-… --no-interactive`（apply 结束后再 `--strict`）
- [x] 2. `parse_slash_command`：`session-tree` / `session-fork`；移除 `tree` / `fork` token
- [x] 3. `product_slash_commands` + unknown 提示串更新
- [x] 4. harness / 测试中产品 slash 针改为新名；旧名断言 unknown
- [x] 5. design/keybindings/PI_DELTAS 手测备忘中的产品入口名同步
- [x] 6. `just fmt` + 相关 test / clippy；tasks 全勾后再次 validate `--strict`
