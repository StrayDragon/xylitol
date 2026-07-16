# Tasks — c1220-migrate-domain-security-bdd

- [x] delta domain-security：modify r7 + op_scenario network-domain-block（feature:true）
- [x] `llman sdd solidify c1220` 生成 domain-security.feature（1 场景）
- [x] bdd.rs sandbox_bdd mod：新增 2 纯文本 step（given/when）+ 1 #[scenario] 绑定
- [x] `cargo test --test bdd` → 100 passed, 0 failed
- [x] `llman sdd validate c1220 --strict` 通过
