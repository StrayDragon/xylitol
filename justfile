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
# crates are not linted by this gate; use `lint-all` for `--all-targets`.
lint:
    cargo clippy --all-features -- -D warnings

# Clippy on all targets (lib, bins, tests, benches, examples) — local / pre-PR.
lint-all:
    cargo clippy --all-features --all-targets -- -D warnings
    cargo clippy -p xylitol-tui --all-targets -- -D warnings

# Run cargo test (all features so tui/server tests run, not just default `cli`).
test:
    if command -v cargo-nextest >/dev/null; then cargo nextest run --all-features --profile ci; else cargo test --all-features; fi

# Run TUI end-to-end integration tests (layer 5: PTY/tmux). Slow + needs a real
# PTY and/or tmux; gated #[ignore] so they never run under the default `test`.
# Covers xylitol-tui agent_demo + product Fake smoke (`pty_product_*`).
test-tui-e2e:
    cargo test --test tui_e2e -- --ignored

# TUI E2E — portable-pty driver only (no tmux needed).
# Includes agent_demo cases and product Fake smoke (`pty_product_*`, c485/c669).
test-tui-e2e-pty:
    cargo test --test tui_e2e -- --ignored pty

# TUI E2E — tmux driver only (requires the tmux binary on PATH).
test-tui-e2e-tmux:
    cargo test --test tui_e2e -- --ignored tmux

# Run the xylitol-tui agent_demo (package dynamic playground — not product host).
# Experiment shapes/keys here before wiring `src/app/tui`; product hand-test uses
# Fake: `cargo run -- --trust --tui --model fake` with isolated config.
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

# --- scripts/ QA checks -------------------------------------------------
# Convention (normative for this repo):
#   - scripts/check_*.py or scripts/check-*.py = non-mutating gate scripts.
#     Each MUST be reachable from `just qa` (via `check-scripts` / deps).
#   - Other scripts/ files (e.g. cleanup_*) are maintenance tools and MUST NOT
#     be required by `qa` (they may mutate the tree).
#
# Meta-gate: every check_* script path must appear in this justfile, and `qa`
# must depend on `check-scripts-wired` + `check-scripts`.
check-scripts-wired:
    #!/usr/bin/env bash
    set -euo pipefail
    qa_hdr="$(awk '/^qa:/{print; exit}' justfile)"
    if [[ "$qa_hdr" != *check-scripts-wired* ]] || [[ "$qa_hdr" != *check-scripts* ]]; then
        echo "error: just qa must list check-scripts-wired and check-scripts as dependencies" >&2
        echo "  got: $qa_hdr" >&2
        exit 1
    fi
    shopt -s nullglob
    missing=0
    # Wired if the exact path appears, or the check-scripts recipe globs the family.
    has_glob=0
    if grep -E -q 'scripts/check_\*\.py|scripts/check-\*\.py' justfile; then
        has_glob=1
    fi
    for f in scripts/check_*.py scripts/check-*.py; do
        if grep -F -q "$f" justfile || [[ "$has_glob" -eq 1 ]]; then
            continue
        fi
        echo "error: QA check script not wired into justfile (list it or keep check-scripts glob): $f" >&2
        missing=1
    done
    if [[ "$missing" -ne 0 ]]; then
        exit 1
    fi
    echo "ok: scripts/check_* wired into just qa"

# Run every scripts/check_*.py / check-*.py (prefer `--check` when supported).
check-scripts:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    files=(scripts/check_*.py scripts/check-*.py)
    if [[ "${#files[@]}" -eq 0 ]]; then
        echo "ok: no scripts/check_*.py yet"
        exit 0
    fi
    for f in "${files[@]}"; do
        if python3 "$f" --help 2>/dev/null | grep -q -- '--check'; then
            python3 "$f" --check
        else
            python3 "$f"
        fi
    done

# Open DESIGN playground HTML (Linux; xdg-open).
open-design-playground:
    xdg-open src/app/tui/design/playground/index.html

# Package TUI tests with default features (includes highlight) — layers 1–4.
test-tui:
    cargo test -p xylitol-tui

# Unified daily / PR gate (no TUI layer-5 E2E — needs PTY/tmux).
# Order: fmt → clippy → workspace tests → package TUI harness → docs → DESIGN tokens
# → scripts/check_* (wired + run) → prek.
qa: fmt-check lint test test-tui doc-check check-tui-tokens check-scripts-wired check-scripts
    @echo "All checks passed!"
    prek run --all-files

# Full gate including TUI layer-5 E2E (portable-pty + tmux; #[ignore]).
qa-e2e: qa test-tui-e2e
    @echo "qa-e2e passed!"

alias check := qa
alias ci := qa

# fmt-check only (cargo fmt covers all crates regardless of features).
fmt-check:
    cargo fmt --all -- --check

# --- Documentation ---

# Build API docs (cargo doc) and open in browser.
doc:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features --open

# Check API docs build without warnings.
doc-check:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features

# Run doc tests (verify /// examples compile).
doc-test:
    cargo test --doc --all-features
