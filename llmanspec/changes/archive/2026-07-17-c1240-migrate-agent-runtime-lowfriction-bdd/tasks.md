# Tasks — c1240-migrate-agent-runtime-lowfriction-bdd

- [x] delta agent-runtime：modify ar3/ar4/ar5/ar12 + 5 op_scenario（feature:true）
- [x] 同步 main spec：ar3/ar12 的 scenario 文本调整为对齐现有 step（策略二）
- [x] 生成 .feature：agent-runtime.feature 追加 5 场景（continues-after-tools / done-not-turn-end / abort-drops-sse / builder-build / ports-exist）
- [x] bdd.rs 绑定：3 个 #[scenario]（ar3×2、ar12，零新代码）+ 6 个新 step（ar4、ar5，trivial 断言）+ 2 个 #[scenario]（ar4、ar5）
- [x] `cargo test --test bdd -- --test-threads=1` → 105 passed, 0 failed
- [x] `llman sdd validate c1240 --strict` 通过
- [x] `llman sdd validate agent-runtime --check` → 7 features parsed
