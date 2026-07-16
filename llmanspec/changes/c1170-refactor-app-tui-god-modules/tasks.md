# Tasks — c1170-refactor-app-tui-god-modules

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1170-refactor-app-tui-god-modules --no-interactive`
- [x] 2. `host/`：抽出 `input_policy.rs` + `session_ops.rs`；`mod.rs` 显著变薄
- [x] 3. `layout/`：抽出 `slash_catalog.rs` + `root/{slot_input,slot_nav,render}`；`root/mod.rs` 显著变薄
- [x] 4. `effects.rs` → `effects/`：按 slash/pending_ui/bang 切文件；保留单一 `drain_pending`
- [x] 5. `bridge/`：`UiModel` 等迁入 `model.rs`；`mod.rs` 保留 `apply_xy_event` + re-export
- [x] 6. 行数断言：四入口文件均 < 800；`just fmt` + 相关 clippy
- [x] 7. harness（ABS/HRS/BASE 相关）+ `cargo test --test bdd -- --test-threads=1` 中 app-tui 绿
- [x] 8. `llman sdd validate c1170-refactor-app-tui-god-modules --strict --no-interactive`
