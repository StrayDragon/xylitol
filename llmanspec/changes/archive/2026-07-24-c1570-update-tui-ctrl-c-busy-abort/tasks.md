# c1570 Tasks

## 1. Busy Ctrl+C = abort

- [x] 1.1 `try_busy_input`：busy 且无 overlay 时 `app.clear` 与 Esc 同 latch（abort，不 quit）
- [x] 1.2 idle：保留非空清空 / 空退出；overlay：先关槽
- [x] 1.3 harness：busy Ctrl+C abort 且 `!should_quit`；idle 双 Ctrl+C 仍退出
- [x] 1.4 test: `cargo test -p xylitol --lib -- harness_ctrl_c harness_busy`

## 2. 合约

- [x] 2.1 修订 `ati2` + `.feature`（busy Ctrl+C / idle 清空退出）
- [x] 2.2 同步 `design/keybindings.md` Ctrl+C 行
- [x] 2.3 attach + `validate --strict --no-interactive`
