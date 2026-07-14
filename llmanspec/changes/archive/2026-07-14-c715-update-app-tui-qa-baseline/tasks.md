# Tasks — c715-update-app-tui-qa-baseline

- [x] 1. `LLMANSPEC_BASE_REF=origin/main llman sdd validate c715-update-app-tui-qa-baseline --no-interactive`（提案闸；apply 收尾再 `--strict`）
- [x] 2. `Cargo.toml`：`rstest-bdd` / `rstest-bdd-macros` → `0.6.0-beta3`；`cargo update -p rstest-bdd -p rstest-bdd-macros`
- [x] 3. 跑 `cargo test --test bdd -- --test-threads=1`；修升级 break（若有）至核心 feature 全绿
- [x] 4. 抽出共享 bang Esc helper；`mod.rs` 与 harness HRS 改用；去掉第三份手写 select
- [x] 5. 调整 `pump_host_driver`：Esc 敏感 bang 路径走 helper（保留纯完成态 await 若仍需要）
- [x] 6. 新增 `tests/features/app-tui-{abort,bang,esc-overlay,queue}.feature` + step（复用 HostSession/ScriptedDriver/drain_pending）
- [x] 7. 确认 ABS/HRS/相关 B 系列 harness 绿；可选补「Esc 与 drain 之间注入 Xy」测（语义仍为现状基线，不抢 c720）
- [x] 8. `just fmt` + 相关 clippy；再跑全量 bdd + 相关 harness
- [x] 9. 复核 `design.md` BASE 清单与实现一致；`validate --strict` 再跑一遍
