set shell := ["bash", "-euo", "pipefail", "-c"]
# just 1.46+ [arg] attributes (pattern / long options).
set unstable

# Default gate verbosity: agent-friendly quiet.
# Override: `just qa normal` | `just qa verbose` | `JUST_VERBOSITY=normal just qa`
# - quiet: native quiet flags; scripts omit --verbose (silent success, errors still print)
# - normal: human-readable tool defaults; scripts still quiet unless verbose
# - verbose: tool -v where useful; scripts get --verbose
verbosity_default := env("JUST_VERBOSITY", "quiet")

# Pinned toolchain first (rust-toolchain.toml lives in this repo; Homebrew stable
# cargo reports a different version and makes `llman-sdd validate --specs` fail).
export PATH := env_var_or_default("HOME", "/usr") + "/.cargo/bin:" + env_var("PATH")

_default:
    @just --list

# Install prek hooks (+ optional complexity CLI into .tools/).
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    # pre-commit + commit-msg only: prek owns text hygiene and the commit
    # convention; cargo gates live in `just qa` (manual / CI), never hooks.
    prek install -t pre-commit -t commit-msg
    # LFS pre-push is owned by git-lfs's native hooks (pre-push + post-*),
    # NOT by prek. Idempotent; also fixes fresh clones missing hooks.
    git lfs install --local
    if ! command -v cccc-rs >/dev/null && [[ ! -x .tools/bin/cccc-rs ]]; then
      cargo install cccc-rs-cli --version 0.4.0 --locked --root .tools
    fi

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
# Extra compile-off paths that `just lint` (all-features) never builds:
#   - xylitol-tui --no-default-features (highlight stub)
#   - xylitol default minus otel (W1.3b stub; cli/tui/server/mcp still on)
[arg('verbosity', pattern='quiet|normal|verbose')]
lint-all verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)
        cargo clippy -q --all-features --all-targets --message-format=short -- -D warnings
        cargo clippy -q -p xylitol-tui --all-targets --message-format=short -- -D warnings
        cargo check -q -p xylitol-tui --no-default-features --message-format=short
        cargo clippy -q -p xylitol --no-default-features --features cli,tui,server,mcp --message-format=short -- -D warnings
        ;;
      normal)
        cargo clippy --all-features --all-targets -- -D warnings
        cargo clippy -p xylitol-tui --all-targets -- -D warnings
        cargo check -p xylitol-tui --no-default-features
        cargo clippy -p xylitol --no-default-features --features cli,tui,server,mcp -- -D warnings
        ;;
      verbose)
        cargo clippy -v --all-features --all-targets -- -D warnings
        cargo clippy -v -p xylitol-tui --all-targets -- -D warnings
        cargo check -v -p xylitol-tui --no-default-features
        cargo clippy -v -p xylitol --no-default-features --features cli,tui,server,mcp -- -D warnings
        ;;
    esac

# Workspace test matrix (nextest profile agent/ci, or cargo test fallback):
# root package + xylitol-official-plugin-provider-openai lib. NOT in this matrix:
#   - xylitol-tui → sole runner is `just test-tui` (in-process #[serial]
#     coordination, TUI harness methodology SSOT);
#   - lab_responses_prompt_cache → serial-only via `qa-test-live-provider`.
# -E overrides the profile default-filter, so the live-binary clause is
# restated here; ad-hoc `cargo nextest run` keeps the nextest.toml filter.
[arg('verbosity', pattern='quiet|normal|verbose')]
test verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    matrix=(-E 'not (package(xylitol-tui) or binary(lab_responses_prompt_cache))')
    if command -v cargo-nextest >/dev/null; then
      case "{{verbosity}}" in
        quiet)
          # Failures only; keep one Summary line from the agent profile.
          cargo nextest run --workspace --all-features --profile agent \
            "${matrix[@]}" --show-progress none --cargo-quiet \
            --success-output never
          ;;
        normal)
          cargo nextest run --workspace --all-features --profile ci \
            "${matrix[@]}" --show-progress none
          ;;
        verbose)
          cargo nextest run --workspace --all-features --profile ci \
            "${matrix[@]}" --status-level all --final-status-level all
          ;;
      esac
    else
      # Same matrix without nextest: exclude xylitol-tui (test-tui covers it)
      # and the live HTTP suite (parallel cargo test would hammer llama.cpp).
      case "{{verbosity}}" in
        quiet)
          if ! out=$(cargo test -q --workspace --all-features --exclude xylitol-official-plugin-provider-openai --exclude xylitol-tui 2>&1); then
            printf '%s\n' "$out"
            exit 1
          fi
          if ! out=$(cargo test -q -p xylitol-official-plugin-provider-openai --lib 2>&1); then
            printf '%s\n' "$out"
            exit 1
          fi
          ;;
        normal)
          cargo test --workspace --all-features --exclude xylitol-official-plugin-provider-openai --exclude xylitol-tui
          cargo test -p xylitol-official-plugin-provider-openai --lib
          ;;
        verbose)
          cargo test -v --workspace --all-features --exclude xylitol-official-plugin-provider-openai --exclude xylitol-tui
          cargo test -v -p xylitol-official-plugin-provider-openai --lib
          ;;
      esac
    fi

# Live Responses prompt-cache counterexample (dedicated <global-dir>/dev config).
# Strictly serial (--test-threads=1). Always part of default `just qa` (any verbosity).
# Missing/disabled global dev config → skip (pass). enabled=true → must hit gateway.
# CI / offline opt-out: XYLITOL_SKIP_LIVE=1 skips this gate unconditionally
# (env, because `qa` composes this recipe as a dependency — conditional deps
# cannot take just args). Example: `XYLITOL_SKIP_LIVE=1 just qa`.
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-test-live-provider verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "${XYLITOL_SKIP_LIVE:-}" == "1" ]]; then
      echo "live-provider: skipped (XYLITOL_SKIP_LIVE=1)"
      exit 0
    fi
    # One binary, one thread: never fan out concurrent llama.cpp requests from this suite.
    case "{{verbosity}}" in
      quiet)
        # Keep --nocapture so RUN/SKIP lines exist; surface a one-liner so quiet qa
        # still shows the live gate ran (not only `just qa normal`).
        if ! out=$(cargo test -q -p xylitol-official-plugin-provider-openai --test lab_responses_prompt_cache -- --test-threads=1 --nocapture 2>&1); then
          printf '%s\n' "$out"
          exit 1
        fi
        if printf '%s\n' "$out" | rg -q 'live-provider: RUN'; then
          printf '%s\n' "$(printf '%s\n' "$out" | rg 'live-provider: (RUN|cache_read)' | tail -n 2)"
        elif printf '%s\n' "$out" | rg -q 'live-provider: SKIP'; then
          printf '%s\n' "$(printf '%s\n' "$out" | rg 'live-provider: SKIP' | tail -n 1)"
        else
          echo "live-provider: ok"
        fi
        ;;
      normal)  cargo test -p xylitol-official-plugin-provider-openai --test lab_responses_prompt_cache -- --test-threads=1 --nocapture ;;
      verbose) cargo test -v -p xylitol-official-plugin-provider-openai --test lab_responses_prompt_cache -- --test-threads=1 --nocapture ;;
    esac

# Generate configs/testing/live-provider.example.yaml (maintenance; not in qa).
# Live-provider config lives at <global-dir>/dev/live-provider.yaml (shared via
# dotxylitol); the repo only ships this auto-generated example.
gen-live-provider-example:
    python3 scripts/gen_live_provider_example.py

# Generate configs/example.yaml (maintenance; not in qa).
# Edit scripts/gen_config_example.py TEMPLATE, then re-run this recipe.
gen-config-example:
    python3 scripts/gen_config_example.py

# fory-v3 envelope codegen (fbs -> Rust). Maintenance only (needs local apache/fory clone);
# `--check` verifies the checked-in artifact matches the fbs source of truth.
codegen-envelope *args:
    python3 scripts/gen_envelope.py {{ args }}

# Provider-safe MCP tool naming gate (registry SSOT + wire encode on all three APIs).
# Not live-network; safe in qa loops. Prefer this after changing MCP_PUBLIC_DELIMITER.
[arg('verbosity', pattern='quiet|normal|verbose')]
test-provider-safe-tool-names verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    # Note: justfile treats `(...)` specially — keep filter lists as space-separated strings.
    qflag=(); vflag=()
    case "{{verbosity}}" in
      quiet) qflag=(-q) ;;
      verbose) vflag=(-v) ;;
    esac
    main_filters="tool_name:: provider_safe_names:: test_mcp_tool_adapter_name_format adapters_from_discovered_names"
    openai_filters="tool_wire:: build_body_mcp_tool_names_are_provider_safe completions_mcp_tool_names_are_provider_safe"
    anthropic_filters="anthropic_mcp_tool_names_are_provider_safe"
    for f in $main_filters; do
      cargo test "${qflag[@]}" "${vflag[@]}" --lib --all-features -- "$f"
    done
    for f in $openai_filters; do
      cargo test "${qflag[@]}" "${vflag[@]}" -p xylitol-official-plugin-provider-openai --lib -- "$f"
    done
    for f in $anthropic_filters; do
      cargo test "${qflag[@]}" "${vflag[@]}" -p xylitol-official-plugin-provider-anthropic --lib -- "$f"
    done

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

# Engine demos live in crates/xylitol-tui/justfile (just demo-tui there).

# Fail if package Palette hex diverges from DESIGN.md frontmatter.
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-check-tui-tokens verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    args=(--check)
    if [[ "{{verbosity}}" == "verbose" ]]; then
      args+=(--verbose)
    fi
    python3 scripts/check_tui_tokens.py "${args[@]}"

# Resolve cccc-rs: XYLITOL_CCCC_RS, PATH, .tools/, else cargo install (crates.io latest pin).
[private]
_cccc-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${XYLITOL_CCCC_RS:-}" && -x "${XYLITOL_CCCC_RS}" ]]; then
      printf '%s' "${XYLITOL_CCCC_RS}"
    elif command -v cccc-rs >/dev/null; then
      command -v cccc-rs
    elif [[ -x .tools/bin/cccc-rs ]]; then
      printf '%s' .tools/bin/cccc-rs
    else
      cargo install cccc-rs-cli --version 0.4.0 --locked --root .tools >/dev/null
      printf '%s' .tools/bin/cccc-rs
    fi

# HARD: ath12 TUI entry coordinators. Thresholds = src/AGENTS.md「复杂度与体量」.
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-check-complexity verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just _cccc-bin)"
    entries=(
      crates/xylitol-app-tui/src/host/mod.rs
      crates/xylitol-app-tui/src/layout/root/mod.rs
      crates/xylitol-app-tui/src/effects/mod.rs
      crates/xylitol-app-tui/src/bridge/mod.rs
    )
    if [[ "{{verbosity}}" == "quiet" ]]; then
      "$bin" --max-cognitive 32 --max-cyclomatic 27 "${entries[@]}" >/dev/null
    else
      "$bin" --max-cognitive 32 --max-cyclomatic 27 --table --min 8 "${entries[@]}"
    fi

# Soft complexity radar (cccc-rs top-cognitive). Not a hard gate.
[arg('verbosity', pattern='quiet|normal|verbose')]
complexity verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just _cccc-bin)"
    "$bin" --table --top-cognitive 20 --min 10 \
      --exclude='**/tests.rs' --exclude='**/tests/**' --exclude='**/harness.rs' \
      src
    echo "(radar soft — not a qa hard gate; entry limits cognitive≤32 cyclomatic≤27)"

# --- scripts/ QA checks -------------------------------------------------
# Convention (normative for this repo):
#   - scripts/check_*.py or scripts/check-*.py = non-mutating gate scripts.
#     Each MUST be reachable from `just qa` (via `qa-check-scripts` / deps).
#     Own `--verbose` for ok summaries; default silent success.
#   - Other scripts/ files (e.g. cleanup_*) are maintenance tools and MUST NOT
#     be required by `qa` (they may mutate the tree).
#
# Meta-gate: every check_* script path must appear in this justfile, and `qa`
# must depend on `qa-check-scripts-wired` + `qa-check-scripts`.
# Complexity HARD gate: `qa-check-complexity` (cccc-rs on ath12 entry mods).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-check-scripts-wired verbosity=verbosity_default:
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
    if [[ "$qa_hdr" != *qa-check-scripts-wired* ]] || [[ "$qa_hdr" != *qa-check-scripts* ]]; then
        echo "error: just qa must list qa-check-scripts-wired and qa-check-scripts as dependencies" >&2
        echo "  got: $qa_hdr" >&2
        exit 1
    fi
    shopt -s nullglob
    files=(scripts/check_*.py scripts/check-*.py)
    if [[ "${#files[@]}" -eq 0 ]]; then
        echo "error: no scripts/check_*.py on disk; gate must not silently run zero checks" >&2
        exit 1
    fi
    body="$(awk '/^qa-check-scripts([ :]|$)/ {grab=1; next} grab {if ($0 ~ /^[ \t]/) print; else exit}' justfile)"
    if ! grep -Eq 'files=\(scripts/check_\*\.py scripts/check-\*\.py\)' <<<"$body" \
        || ! grep -Fq '"${files[@]}"' <<<"$body"; then
        echo "error: qa-check-scripts recipe must keep glob wiring:" >&2
        echo '  files=(scripts/check_*.py scripts/check-*.py) + iterate "${files[@]}"' >&2
        echo "  (explicit per-file whitelists fork from the glob and strand new checks)" >&2
        exit 1
    fi
    if [[ "{{verbosity}}" == "verbose" ]]; then
        echo "ok: qa-check-scripts keeps glob wiring over ${#files[@]} check script(s)"
    fi

# Run every scripts/check_*.py / check-*.py (prefer `--check` when supported).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-check-scripts verbosity=verbosity_default:
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
        # ${arr[@]+…} 形态：bash 3.2（macOS 自带）在 `set -u` 下不将空数组当 unbound。
        if python3 "$f" --help 2>/dev/null | grep -q -- '--check'; then
            python3 "$f" --check ${extra[@]+"${extra[@]}"}
        else
            python3 "$f" ${extra[@]+"${extra[@]}"}
        fi
    done

# Regenerate the skill description brief tail from references/*.md frontmatter
# (drift gate: scripts/check_skill_description.py --check, wired via qa-check-scripts).
gen-skill-description:
    python3 scripts/gen_skill_description.py

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
# Order: fmt → clippy → workspace tests → live-provider (serial) → package TUI
# harness → docs (qa-doc-check + qa-doc-test) → DESIGN tokens → scripts/check_* →
# prek (text-hygiene-only hooks; cargo gates never in prek — see AGENTS.md).
# Default verbosity=quiet (agent-friendly). Pass `normal` / `verbose` for humans.
# Skip options (env): XYLITOL_SKIP_LIVE=1 skips the live-provider gate (CI).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa verbosity=verbosity_default: \
    (qa-fmt-check verbosity) \
    (lint verbosity) \
    (test verbosity) \
    (qa-test-live-provider verbosity) \
    (test-tui verbosity) \
    (qa-doc-check verbosity) \
    (qa-doc-test verbosity) \
    (qa-check-tui-tokens verbosity) \
    (qa-check-complexity verbosity) \
    (qa-check-scripts-wired verbosity) \
    (qa-check-scripts verbosity) \
    (qa-spec-validate verbosity)
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

# SDD contract sweep: index rebuild + strict validate (part of `just qa`) and the
# review counters. `index rebuild` first — a stale index inflates warning counts.
sdd:
    llman-sdd index rebuild
    @llman-sdd validate --all --strict
    @llman-sdd review

# Strict specs/change validate (wired into `just qa`; deterministic, no network).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-spec-validate verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    # CI runner 不装 llman-sdd（dev/SDD 工具）：有则门禁强制（失败即红），
    # 缺失则显式跳过并说明——spec 行为完整性由 qa 内 BDD/tests 全量闸兜底。
    if ! command -v llman-sdd >/dev/null 2>&1; then
      echo "qa-spec-validate: skipped (llman-sdd 未安装；CI 由 BDD/test 闸兜底)"
      exit 0
    fi
    if [[ "{{verbosity}}" == "quiet" ]]; then
      llman-sdd validate --all --strict | tail -n 8
    else
      llman-sdd validate --all --strict
    fi

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

# qa-fmt-check only (cargo fmt covers all crates regardless of features).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-fmt-check verbosity=verbosity_default:
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
# Build-only: `python3 scripts/profile_tui_suite.py --build`.
# Summarize: `python3 scripts/summarize_samply_profile.py <run-dir> --addr2line ./target/release/xylitol`.

# Run Fake/tmux scenarios A–E (or subset). Writes target/profile/<run-id>/.
#   just profile-suite
#   just profile-suite "A,D" 15
# c1505 T0d (long history + stream): python3 scripts/profile_tui_suite.py --scenarios E --b-pairs 400 --duration 20 --run-id e-hist-400
profile-suite scenarios="A,B,C,D" duration="20":
    python3 scripts/profile_tui_suite.py --build --scenarios {{scenarios}} --duration {{duration}}

# --- Cargo worktree target isolation (maintenance; not in qa) ---
# Per-worktree CARGO_TARGET_DIR under ~/.cache/cargo-targets/…
# Docs: root AGENTS.md "Worktree 并行开发" section / scripts/cargo_worktree_env.sh
# Usage: eval "$(just cargo-wt-env)"   or   source scripts/cargo_worktree_env.sh
cargo-wt-env:
    #!/usr/bin/env bash
    set -euo pipefail
    scripts/cargo_worktree_env.sh --print

# --- Observability / provider-trace inspect (maintenance; not in qa) ---
# Token-efficient summaries. Skill: xylitol-dev (references/observability.md).
# Filters (--since / --request-id / --turn-id) and rarer subcommands
# (requests / turns / lag / lifecycle / channel): pass via the python CLI, e.g.
#   python3 scripts/inspect_provider_trace.py requests -n 8

obs-summary:
    python3 scripts/inspect_provider_trace.py summary

# Prefer positional: `just obs-recent 80` (Just 1.57 treats `n=80` as a literal arg value).
obs-recent n="40":
    #!/usr/bin/env bash
    set -euo pipefail
    n="{{n}}"; n="${n#n=}"
    python3 scripts/inspect_provider_trace.py recent -n "$n"

# Spinner / host-loop freeze breadcrumbs in xylitol.log (target xylitol::lag).
# Repro: XYLITOL_DEBUG=1 or RUST_LOG=xylitol::lag=info,xylitol=warn — then submit a prompt
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
    rg -n "xylitol::lag|xylitol::wheel|host_run_await|host_drain_pending|host_mcp_poll|mcp_settle_|rebuild_system_prompt|run_ensure_session|run_load_history|run_build_tool_schemas|host_tick|host_wheel_paint|host_step_paint" "$log" | tail -n "{{n}}"

# --- Documentation ---

# Build API docs (cargo doc) and open in browser.
doc:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features --open

# Check API docs build without warnings.
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-doc-check verbosity=verbosity_default:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{verbosity}}" in
      quiet)   RUSTDOCFLAGS='-D warnings' cargo doc -q --no-deps --all-features ;;
      normal)  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features ;;
      verbose) RUSTDOCFLAGS='-D warnings' cargo doc -v --no-deps --all-features ;;
    esac

# Run doc tests (verify /// examples compile).
[arg('verbosity', pattern='quiet|normal|verbose')]
qa-doc-test verbosity=verbosity_default:
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
