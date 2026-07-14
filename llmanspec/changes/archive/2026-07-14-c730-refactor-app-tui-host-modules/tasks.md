# Tasks — c730-refactor-app-tui-host-modules

- [x] 1. 确认 c725 已 archive；`validate c730-… --no-interactive`
- [x] 2. 引入 `PendingOps`（`host/pending.rs`）；迁移 `pending_*` 字段与 take_* API
- [x] 3. 抽出 input 策略中 ati32 busy-bang 硬拒；补 harness
- [x] 4. host.rs → `host/mod.rs` 模块化；保证无 Driver 进 widgets
- [x] 5. travel/fork `get_messages` 错误表面化（ath13）
- [x] 6. commands 多文件整理（本变更保持 commands.rs；atm7 审计通过）
- [x] 7. BASE + 新测 + bdd；fmt
- [x] 8. `validate --strict`；准备 archive
