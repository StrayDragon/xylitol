//! BDD step registry — dedicated integration-test binary (c2837).
//!
//! 编译隔离：原 BDD 经 `src/tests.rs` 的 `mod bdd` 挂入 lib 测试配置，全部 BDD
//! 场景宏展开随每次 lib 测试编译（实测占 lib 测试编译 ≈53%，43s→20s 基线）。
//! 独立 target 后：
//!   - `cargo test --test bdd` 真实执行全部 BDD 场景（test-bdd r37 名实相符）；
//!   - BDD 侧改动只重编本 binary，与 main lib 编译解耦。
//!
//! 可见性约定：
//!   - steps/bindings 经 `xylitol::…` 访问 lib（`agent`/`infra` 由其
//!     `pub(crate)` 升为 `pub`，纯可见性扩张，见 src/lib.rs）；
//!   - BDD 模块树内部引用保持 `crate::bdd::…`（本 target 的 `mod bdd`）。
//!
//! suite.rs 经 `#[path]` 挂载，`tests/bdd/**` 文件布局原样保留。

#[path = "bdd/suite.rs"]
mod bdd;
