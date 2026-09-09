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
// BDD step registry must be one binary: the whole `tests/bdd` tree mounts as
// one module; its internal `crate::` paths refer to `crate::bdd`.
#[cfg(test)]
#[path = "../tests/bdd/suite.rs"]
mod bdd;

#[cfg(test)]
#[path = "../tests/inner/provider_http_stream_abort.rs"]
mod provider_http_stream_abort;

// Converted criterion bench (was `[[bench]] token_estimator`): benches are
// separate crates and cannot reach `pub(crate)` items either. Run stats via
// `cargo test -r token_estimator -- --nocapture` (Criterion reads args).
#[cfg(test)]
#[path = "../tests/inner/token_estimator_bench.rs"]
mod token_estimator_bench;
