// Unit-test-only module wiring.
//
// Shared test infrastructure lives under `tests/support/` so we can treat it like a lightweight
// "test-support crate" in this single-crate repository.
//
// Layering (agent ↛ infra, surfaces via seams) is normative in `src/AGENTS.md` and
// `write-surface` — enforced by review and seam/behavior tests, not by source-grep
// meta-tests. See llmanspec `layer-architecture` (no arch_guard).

#[path = "../tests/support/mod.rs"]
pub mod support;
