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

# Run TUI end-to-end integration tests (c405 layer 5). Slow + needs a real PTY
# and/or tmux; gated #[ignore] so they never run under the default `test`.
test-tui-e2e:
    cargo test --test tui_e2e -- --ignored

# TUI E2E — portable-pty driver only (no tmux needed).
test-tui-e2e-pty:
    cargo test --test tui_e2e -- --ignored pty

# TUI E2E — tmux driver only (requires the tmux binary on PATH).
test-tui-e2e-tmux:
    cargo test --test tui_e2e -- --ignored tmux

# Run the xylitol-tui agent_demo (default features include syntect highlight).
# Product TUI live playground — experiment shapes here before host wiring.
demo-tui:
    cargo run -p xylitol-tui --example agent_demo

# agent_demo without syntect (lighter / no-highlight regression).
demo-tui-no-highlight:
    cargo run -p xylitol-tui --example agent_demo --no-default-features

# Sync playground tokens.css/js from src/app/tui/DESIGN.md frontmatter.
sync-tui-tokens:
    python3 src/app/tui/design/playground/sync_tokens.py

# Fail if playground tokens or package Palette diverge from DESIGN.md.
check-tui-tokens:
    python3 src/app/tui/design/playground/sync_tokens.py --check

# Open DESIGN playground HTML (Linux; xdg-open).
open-design-playground:
    xdg-open src/app/tui/design/playground/index.html

# Package TUI tests with default features (includes highlight).
test-tui:
    cargo test -p xylitol-tui

# Run all checks (qa = fmt-check + lint + test + doc-check + design tokens).
qa: fmt-check lint test doc-check check-tui-tokens
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
