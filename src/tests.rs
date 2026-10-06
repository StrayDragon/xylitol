// Unit-test-only module wiring.
//
// Shared test infrastructure lives under `tests/support/` so we can treat it like a lightweight
// "test-support crate" in this single-crate repository.
//
// Layering (agent ↛ infra, surfaces via seams) is normative in `src/AGENTS.md` and
// `l8ng-write-surface` — enforced by review and seam/behavior tests, not by source-grep
// meta-tests. See llmanspec `layer-architecture` (no arch_guard).

#[path = "../tests/support/mod.rs"]
pub mod support;

// In-crate test suites. `tests/` integration targets are separate compile
// units and cannot see `pub(crate)` internals, so suites that exercise
// internals are mounted here (files stay physically under `tests/`).
// BDD 已迁出为独立集成测试 target（tests/bdd.rs，c2837 编译隔离）：
// lib 测试编译不再连带 BDD 场景宏展开（实测该阶段缩减 ≈53%）。

#[cfg(test)]
#[path = "../tests/inner/provider_http_stream_abort.rs"]
mod provider_http_stream_abort;

// Converted criterion bench (was `[[bench]] token_estimator`): benches are
// separate crates and cannot reach `pub(crate)` items either. Run stats via
// `cargo test -r token_estimator -- --nocapture` (Criterion reads args).
#[cfg(test)]
#[path = "../tests/inner/token_estimator_bench.rs"]
mod token_estimator_bench;

// c2810 lab: real-session compaction replay (env-gated, `#[ignore]`, not in qa).
#[cfg(test)]
#[path = "../tests/inner/lab_compaction_replay.rs"]
mod lab_compaction_replay;
