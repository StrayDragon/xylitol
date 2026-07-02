set shell := ["bash", "-euo", "pipefail", "-c"]

_default:
    @just --list

# Install prek hooks.
setup:
    prek install

# Run cargo fmt (write).
fmt:
    cargo fmt

# Run cargo clippy with warnings denied (lib + bins, all features so the tui
# and server code paths are linted, not just default `cli`). Tests/integration
# crates are not linted by this gate (run `cargo clippy --all-targets` to
# inspect them).
lint:
    cargo clippy --all-features -- -D warnings

# Run cargo test (all features so tui/server tests run, not just default `cli`).
test:
    if command -v cargo-nextest >/dev/null; then cargo nextest run --all-features --profile ci; else cargo test --all-features; fi

# Run all checks (qa = fmt-check + lint + test + doc-check).
qa: fmt-check lint test doc-check
    @echo "All checks passed!"
    prek run --all-files

alias check := qa
alias ci := qa

# fmt-check only (cargo fmt covers all crates regardless of features).
fmt-check:
    cargo fmt --all -- --check

# --- Documentation ---

# Build API docs (cargo doc) and open in browser.
doc:
    cargo doc --no-deps --all-features --open

# Check API docs build without errors.
doc-check:
    cargo doc --no-deps --all-features

# Run doc tests (verify /// examples compile).
doc-test:
    cargo test --doc --all-features
