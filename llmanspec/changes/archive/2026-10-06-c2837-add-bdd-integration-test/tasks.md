# Tasks — c2837 BDD 独立集成测试目标

- [x] 撰写 proposal / design / tasks（本文件）；决策预览 D1–D5 向用户展示
- [x] specs：`test-bdd` r37 verified-by 更新 + 新增 r1913 规则（独立 target 承载不变量）并 commit
- [x] 实施：新增 `tests/bdd.rs` 入口（`#[path] mod bdd`）
- [x] 实施：`src/tests.rs` 移除 bdd 挂载
- [x] 实施：`src/lib.rs` agent/infra/utils 可见性升格（含动机注释）
- [x] 实施：机械改写 `tests/bdd/**` 引用（`crate::tests::bdd`→`crate::bdd`，`crate::<外部>`→`xylitol::<外部>`）
- [x] 编译修错迭代：`cargo test --test bdd --no-run` 到通过，946 场景全绿
- [x] 验证：`just qa` 全绿（矩阵含 binary(bdd)）+ `check_bdd_literal_bytes` + validate --strict
- [x] 编译基线复核：`cargo test --lib --no-run` ≈20s
