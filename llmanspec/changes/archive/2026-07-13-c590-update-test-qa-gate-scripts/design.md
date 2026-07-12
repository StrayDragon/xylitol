# Design — c590-update-test-qa-gate-scripts

## Decision

Align `test-qa-gate` with the already-shipped justfile convention:

- `scripts/check_*.py` / `check-*.py` → qa via `check-scripts-wired` + `check-scripts`
- `cleanup_*` → maintenance only

No further code changes required (f556d34).
