# c2071 PTY / demo smoke

Date: 2026-08-12
Branch: `sdd/c2071-update-app-tui-host-mode-b-only`
Cargo target isolation: every command that could invoke Cargo was prefixed with
`eval "$(just cargo-wt-env)"`.

## Summary

| Result | Count |
|---|---:|
| PASS | 6 |
| FAIL | 0 |
| SKIP | 2 |

Environment: `/bin/tmux`, `/bin/script`, and `/bin/timeout` were available.

## Results

| Item | Result | Evidence |
|---|---|---|
| Recipe discovery | PASS | TUI fast, PTY, tmux, combined E2E, ApplicationOwned demo, and host-loop recipes exist. |
| Package TUI layers 1–4 | PASS | `just test-tui normal`; all test binaries and doctests passed. |
| Portable PTY E2E | PASS | 24 passed, 0 failed, including product Fake smokes and all `pty_agent_demo_alt_*` cases. |
| tmux E2E | PASS | 6 passed, 0 failed. |
| c2071 product default | PASS | Exact app tests confirm `TuiRunOptions::default()` and product constructors bind and begin `ApplicationOwned`, with mouse capture and dock registration. |
| Product binary AO lifecycle | PASS | Ephemeral raw-PTY probe, with `XYLITOL_TUI_MOUSE` absent, observed alt enter, mouse enable, clean exit, mouse disable, alt leave, and Fake transcript dump after leave-alt. |
| Interactive `just demo-tui-alt-screen` | SKIP | Deliberately not launched because it is interactive; its non-interactive ignored PTY equivalents passed. |
| Combined `just test-tui-e2e` | SKIP | Split recipes covered the same set: 24 PTY + 6 tmux tests all passed. |

## Commands and exit codes

1. `just --list | rg -i 'tui|e2e|demo|pty|alt'` — exit `0`.
2. `command -v tmux; command -v script; command -v timeout; rustc --version` — exit `0`.
3. `eval "$(just cargo-wt-env)" && just test-tui normal` — exit `0`.
4. `eval "$(just cargo-wt-env)" && just test-tui-e2e-pty normal` — exit `0`
   (`24 passed; 0 failed`).
5. `eval "$(just cargo-wt-env)" && cargo test --lib interaction_mode_defaults_to_application_owned -- --exact && just test-tui-e2e-tmux normal`
   — exit `0`; the first filter matched zero tests because `--exact` required the
   module-qualified name, while tmux ran `6 passed; 0 failed`.
6. `eval "$(just cargo-wt-env)" && cargo test --lib app::tui::tests::interaction_mode_defaults_to_application_owned -- --exact && cargo test --lib app::tui::tests::interaction_application_owned_at_construction_registers_dock -- --exact`
   — exit `0` (`2 passed; 0 failed` across the two invocations).
7. First ephemeral Python PTY probe around `cargo run --quiet -- tui --trust --model fake`
   — exit `1`: the probe used a malformed byte needle for the Unicode product
   footer and timed out. This was a probe bug, not a product failure; it reached
   neither a failing product assertion nor a c2071 behavior conclusion.
8. Corrected ephemeral Python PTY probe, executing the already-built product
   binary and waiting on `CSI ?1049h` — exit `0`. It asserted:
   - `XYLITOL_TUI_MOUSE` removed from the child environment;
   - `CSI ?1049h` alt-screen enter;
   - `CSI ?1000h` or `CSI ?1003h` mouse enable;
   - Fake reply after submitting `hi`;
   - `/exit` status `0`;
   - `CSI ?1000l` or `CSI ?1003l` mouse disable;
   - `CSI ?1049l` alt-screen leave;
   - `Hello from fake provider` present after leave-alt (exit dump).

## Failure assessment

No product or repository test failed. The sole non-zero command was the first
ad-hoc probe attempt; its root cause was an incorrectly encoded readiness
needle in the probe itself and is outside c2071 product scope. The corrected
probe passed immediately against the same checkout.

## Suggested follow-ups

- Promote the successful product raw-PTY lifecycle assertions into an ignored
  `pty_product_*` regression test so ApplicationOwned startup/teardown is
  covered by `just test-tui-e2e-pty` without an ad-hoc probe.
- Keep `just demo-tui-alt-screen` as a human visual/selection check when an
  interactive terminal review is desired; it is not needed for this automated
  smoke signature.
