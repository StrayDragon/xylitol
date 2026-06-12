# Testing & CI tiers

This repo follows a 4-tier testing pyramid (fast → slow). `just qa` is the default verification entrypoint.

## Tier 1 — Fast unit / no heavy deps

- Runs: unit tests + lightweight integration tests
- No VT100 rendering, no PTY E2E, no network

Commands:

- `cargo test`
- (optional) `cargo nextest run --profile ci`

## Tier 2 — Cross-module harness / mock LLM

- Runs: full agent loop wired via `tests/support/` (`FauxProvider` + `TestHarness`)

Commands:

- `cargo test -p xylitol --lib tests::support`

## Tier 3 — PTY E2E (process-level)

- Runs: spawn the binary in a pseudo-terminal and assert interactive flows
- Feature-gated: `dev-e2e`

Commands:

- `cargo test -p xylitol --features dev-e2e`
