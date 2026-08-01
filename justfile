set shell := ["bash", "-euo", "pipefail", "-c"]
# just 1.46+ [arg] attributes (pattern / long options).
set unstable

# Default gate verbosity: agent-friendly quiet.
# Override: `just qa normal` | `just qa verbose` | `JUST_VERBOSITY=normal just qa`
# - quiet: native quiet flags; scripts omit --verbose (silent success, errors still print)
# - normal: human-readable tool defaults; scripts still quiet unless verbose
# - verbose: tool -v where useful; scripts get --verbose
verbosity_default := env("JUST_VERBOSITY", "quiet")

_default:
    @just --list

# Install prek hooks.
setup:
    prek install

# Run cargo fmt (write).
fmt:
    cargo fmt

# Clippy with warnings denied (lib + bins, all features).
[arg('verbosity', pattern='quiet|normal|verbose')]
lint verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   cargo clippy -q --all-features --message-format=short -- -D warnings ;;
      normal)  cargo clippy --all-features -- -D warnings ;;
      verbose) cargo clippy -v --all-features -- -D warnings ;;
    esac

# Clippy on all targets — local / pre-PR.
[arg('verbosity', pattern='quiet|normal|verbose')]
lint-all verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)
        cargo clippy -q --all-features --all-targets --message-format=short -- -D warnings
        cargo clippy -q -p xylitol-tui --all-targets --message-format=short -- -D warnings
        ;;
      normal)
        cargo clippy --all-features --all-targets -- -D warnings
        cargo clippy -p xylitol-tui --all-targets -- -D warnings
        ;;
      verbose)
        cargo clippy -v --all-features --all-targets -- -D warnings
        cargo clippy -v -p xylitol-tui --all-targets -- -D warnings
        ;;
    esac

# Workspace tests (nextest profile agent/ci, or cargo test fallback).
[arg('verbosity', pattern='quiet|normal|verbose')]
test verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v cargo-nextest >/dev/null; then
      case "{{verbosity}}" in
        quiet)
          # Failures only; keep one Summary line from the agent profile.
          cargo nextest run --all-features --profile agent \
            --show-progress none --cargo-quiet \
            --success-output never
          ;;
        normal)
          cargo nextest run --all-features --profile ci --show-progress none
          ;;
        verbose)
          cargo nextest run --all-features --profile ci \
            --status-level all --final-status-level all
          ;;
      esac
    else
      case "{{verbosity}}" in
        quiet)
          # cargo -q already forwards quiet to libtest; do not also pass --quiet.
          if ! out=$(cargo test -q --all-features 2>&1); then
            printf '%s\n' "$out"
            exit 1
          fi
          ;;
        normal)  cargo test --all-features ;;
        verbose) cargo test -v --all-features ;;
      esac
    fi

# TUI end-to-end integration tests (layer 5: PTY/tmux). Slow + needs a real
# PTY and/or tmux; gated #[ignore] so they never run under the default `test`.
[arg('verbosity', pattern='quiet|normal|verbose')]
test-tui-e2e verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   cargo test -q --test tui_e2e -- --ignored ;;
      normal)  cargo test --test tui_e2e -- --ignored ;;
      verbose) cargo test -v --test tui_e2e -- --ignored ;;
    esac

# TUI E2E — portable-pty driver only (no tmux needed).
[arg('verbosity', pattern='quiet|normal|verbose')]
test-tui-e2e-pty verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   cargo test -q --test tui_e2e -- --ignored pty ;;
      normal)  cargo test --test tui_e2e -- --ignored pty ;;
      verbose) cargo test -v --test tui_e2e -- --ignored pty ;;
    esac

# TUI E2E — tmux driver only (requires the tmux binary on PATH).
[arg('verbosity', pattern='quiet|normal|verbose')]
test-tui-e2e-tmux verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   cargo test -q --test tui_e2e -- --ignored tmux ;;
      normal)  cargo test --test tui_e2e -- --ignored tmux ;;
      verbose) cargo test -v --test tui_e2e -- --ignored tmux ;;
    esac

# Run the xylitol-tui agent_demo (package dynamic playground — not product host).
demo-tui:
    cargo run -p xylitol-tui --example agent_demo

# agent_demo with rail entry skin (left bg strip; /entry-style rail).
demo-tui-rail:
    XYLITOL_AGENT_DEMO_ENTRY_STYLE=rail cargo run -p xylitol-tui --example agent_demo

# agent_demo without syntect (lighter / no-highlight regression).
demo-tui-no-highlight:
    cargo run -p xylitol-tui --example agent_demo --no-default-features

# Sync playground tokens.css/js from src/app/tui/DESIGN.md frontmatter.
sync-tui-tokens:
    python3 src/app/tui/design/playground/sync_tokens.py

# Fail if playground tokens or package Palette diverge from DESIGN.md.
[arg('verbosity', pattern='quiet|normal|verbose')]
check-tui-tokens verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    args=(--check)
    if [[ "{{verbosity}}" == "verbose" ]]; then
      args+=(--verbose)
    fi
    python3 src/app/tui/design/playground/sync_tokens.py "${args[@]}"

# --- scripts/ QA checks -------------------------------------------------
# Convention (normative for this repo):
#   - scripts/check_*.py or scripts/check-*.py = non-mutating gate scripts.
#     Each MUST be reachable from `just qa` (via `check-scripts` / deps).
#     Own `--verbose` for ok summaries; default silent success.
#   - Other scripts/ files (e.g. cleanup_*) are maintenance tools and MUST NOT
#     be required by `qa` (they may mutate the tree).
#
# Meta-gate: every check_* script path must appear in this justfile, and `qa`
# must depend on `check-scripts-wired` + `check-scripts`.
[arg('verbosity', pattern='quiet|normal|verbose')]
check-scripts-wired verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    # Recipe may span continued lines: `qa …: \` / `(dep) \` / …
    qa_hdr="$(awk '
      /^\[/ { next }
      /^qa([ :]|$)/ { grab=1 }
      grab {
        line=$0
        sub(/#.*/, "", line)
        print line
        if ($0 !~ /\\[[:space:]]*$/) exit
      }
    ' justfile)"
    if [[ "$qa_hdr" != *check-scripts-wired* ]] || [[ "$qa_hdr" != *check-scripts* ]]; then
        echo "error: just qa must list check-scripts-wired and check-scripts as dependencies" >&2
        echo "  got: $qa_hdr" >&2
        exit 1
    fi
    shopt -s nullglob
    missing=0
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
    if [[ "{{verbosity}}" == "verbose" ]]; then
        echo "ok: scripts/check_* wired into just qa"
    fi

# Run every scripts/check_*.py / check-*.py (prefer `--check` when supported).
[arg('verbosity', pattern='quiet|normal|verbose')]
check-scripts verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    files=(scripts/check_*.py scripts/check-*.py)
    if [[ "${#files[@]}" -eq 0 ]]; then
        if [[ "{{verbosity}}" == "verbose" ]]; then
            echo "ok: no scripts/check_*.py yet"
        fi
        exit 0
    fi
    extra=()
    if [[ "{{verbosity}}" == "verbose" ]]; then
      extra+=(--verbose)
    fi
    for f in "${files[@]}"; do
        if python3 "$f" --help 2>/dev/null | grep -q -- '--check'; then
            python3 "$f" --check "${extra[@]}"
        else
            python3 "$f" "${extra[@]}"
        fi
    done

# Open DESIGN playground HTML (Linux; xdg-open).
open-design-playground:
    xdg-open src/app/tui/design/playground/index.html

# Package TUI tests with default features (includes highlight) — layers 1–4.
[arg('verbosity', pattern='quiet|normal|verbose')]
test-tui verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)
        # Swallow pass/progress noise; print full output only on failure.
        # Kitty flag races are serialized inside with_kitty_protocol_active (keys.rs).
        if ! out=$(cargo test -q -p xylitol-tui 2>&1); then
          printf '%s\n' "$out"
          exit 1
        fi
        ;;
      normal)  cargo test -p xylitol-tui ;;
      verbose) cargo test -v -p xylitol-tui ;;
    esac

# Unified daily / PR gate (no TUI layer-5 E2E — needs PTY/tmux).
# Order: fmt → clippy → workspace tests → package TUI harness → docs → DESIGN tokens
# → scripts/check_* (wired + run) → prek.
# Default verbosity=quiet (agent-friendly). Pass `normal` / `verbose` for humans.
[arg('verbosity', pattern='quiet|normal|verbose')]
qa verbosity=verbosity_default: \
    (fmt-check verbosity) \
    (lint verbosity) \
    (test verbosity) \
    (test-tui verbosity) \
    (doc-check verbosity) \
    (check-tui-tokens verbosity) \
    (check-scripts-wired verbosity) \
    (check-scripts verbosity)
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)
        prek -q run --all-files
        ;;
      normal)
        echo "All checks passed!"
        prek run --all-files
        ;;
      verbose)
        echo "All checks passed!"
        prek -v run --all-files
        ;;
    esac

# Full gate including TUI layer-5 E2E (portable-pty + tmux; #[ignore]).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-e2e verbosity=verbosity_default: (qa verbosity) (test-tui-e2e verbosity)
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{verbosity}}" != "quiet" ]]; then
      echo "qa-e2e passed!"
    fi

alias check := qa
alias ci := qa

# fmt-check only (cargo fmt covers all crates regardless of features).
[arg('verbosity', pattern='quiet|normal|verbose')]
fmt-check verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   cargo fmt -q --all -- --check ;;
      normal)  cargo fmt --all -- --check ;;
      verbose) cargo fmt -v --all -- --check ;;
    esac

# --- Dev CPU profiling suite (c1520; maintenance; NOT in qa) ---
# Needs: samply, tmux, perf_event_paranoid<=1
#   echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid

# Release build with symbols (strip=none, line-tables-only).
profile-build:
    python3 scripts/profile_tui_suite.py --build

# Run Fake/tmux scenarios A–E (or subset). Writes target/profile/<run-id>/.
#   just profile-suite
#   just profile-suite "A,D" 15
# c1505 T0d (long history + stream): python3 scripts/profile_tui_suite.py --scenarios E --b-pairs 400 --duration 20 --run-id e-hist-400
profile-suite scenarios="A,B,C,D" duration="20":
    python3 scripts/profile_tui_suite.py --build --scenarios {{scenarios}} --duration {{duration}}

# Summarize one profile (xylitol-only filter).
profile-summary path:
    python3 scripts/summarize_samply_profile.py "{{path}}" --addr2line ./target/release/xylitol

# --- Observability / provider-trace inspect (maintenance; not in qa) ---
# Token-efficient summaries. Skill: xylitol-inspect-runtime-logs.
# Filters (--since / --request-id / --turn-id): pass via python CLI, not just kwargs
#   python3 scripts/inspect_provider_trace.py --since 30m summary

obs-summary:
    python3 scripts/inspect_provider_trace.py summary

obs-requests n="8":
    python3 scripts/inspect_provider_trace.py requests -n {{n}}

obs-recent n="40":
    python3 scripts/inspect_provider_trace.py recent -n {{n}}

obs-turns n="8":
    python3 scripts/inspect_provider_trace.py turns -n {{n}}

obs-lag REQUEST_ID="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "{{REQUEST_ID}}" ]]; then
      python3 scripts/inspect_provider_trace.py --request-id "{{REQUEST_ID}}" lag
    else
      python3 scripts/inspect_provider_trace.py lag
    fi

obs-lifecycle REQUEST_ID="" TURN_ID="":
    #!/usr/bin/env bash
    set -euo pipefail
    args=()
    [[ -n "{{REQUEST_ID}}" ]] && args+=(--request-id "{{REQUEST_ID}}")
    [[ -n "{{TURN_ID}}" ]] && args+=(--turn-id "{{TURN_ID}}")
    python3 scripts/inspect_provider_trace.py "${args[@]}" lifecycle

obs-channel REQUEST_ID="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "{{REQUEST_ID}}" ]]; then
      python3 scripts/inspect_provider_trace.py --request-id "{{REQUEST_ID}}" channel
    else
      python3 scripts/inspect_provider_trace.py channel
    fi

# Spinner / host-loop freeze breadcrumbs in xylitol.log (target xylitol::lag).
# Repro: XYLITOL_DEBUG=1 or RUST_LOG=xylitol::lag=info,xylitol=info — then submit a prompt
# while MCP is still connecting; run `just obs-tui-lag` and look for host_run_await /
# mcp_settle_* / rebuild_system_prompt lines ≥80ms.
obs-tui-lag n="80" LOG="":
    #!/usr/bin/env bash
    set -euo pipefail
    log="${LOG:-${XYLITOL_AGENT_DIR:-$HOME/.xylitol}/logs/xylitol.log}"
    if [[ ! -f "$log" ]]; then
      echo "missing log: $log" >&2
      echo "hint: XYLITOL_DEBUG=1 cargo run …  (or RUST_LOG=xylitol::lag=info)" >&2
      exit 1
    fi
    echo "# $log (last {{n}} xylitol::lag lines)"
    rg -n "xylitol::lag|host_run_await|host_drain_pending|host_mcp_poll|mcp_settle_|rebuild_system_prompt|run_ensure_session|run_load_history|run_build_tool_schemas|host_tick" "$log" | tail -n "{{n}}"

# --- Documentation ---

# Build API docs (cargo doc) and open in browser.
doc:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features --open

# Check API docs build without warnings.
[arg('verbosity', pattern='quiet|normal|verbose')]
doc-check verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   RUSTDOCFLAGS='-D warnings' cargo doc -q --no-deps --all-features ;;
      normal)  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features ;;
      verbose) RUSTDOCFLAGS='-D warnings' cargo doc -v --no-deps --all-features ;;
    esac

# Run doc tests (verify /// examples compile).
[arg('verbosity', pattern='quiet|normal|verbose')]
doc-test verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)
        if ! out=$(cargo test -q --doc --all-features 2>&1); then
          printf '%s\n' "$out"
          exit 1
        fi
        ;;
      normal)  cargo test --doc --all-features ;;
      verbose) cargo test -v --doc --all-features ;;
    esac
