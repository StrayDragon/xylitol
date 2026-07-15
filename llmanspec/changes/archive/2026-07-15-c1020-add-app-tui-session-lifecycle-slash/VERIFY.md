# VERIFY — c1020-add-app-tui-session-lifecycle-slash

Date: 2026-07-15

## Gate

| Check | Result |
|---|---|
| tasks all `[x]` | PASS |
| validate `--strict` | PASS |
| harness h33/h34/h35 | PASS |
| arch_guard | PASS |
| `just lint` (`clippy -D warnings`) | PASS |
| `just fmt` | PASS |

## Spec ↔ code

| Req | Evidence |
|---|---|
| atm11 `/session-new` | `PendingSlash::SessionNew` → `Driver::new_session` → `apply_new_session`；带参 Usage |
| atm11 `/session-clone` | leaf 上 `fork_session(..., At)` + switch → `apply_clone_session`；无 leaf 「Nothing to clone yet」；≠ session-fork Before/At |
| atm11 `/session-name` | 无参 get / usage；有参 `set_session_name`（sanitize）+ normalize 提示 |
| atm11 旧名 | `/new` `/clone` `/name` → parse `None` → unknown |
| ath15 seam | `Driver::{new_session,get_session_name,set_session_name}`；store 默认 `get/set_session_name`；TUI 无 infra import |
| A07 | `PI_DELTAS.md` |

## CRITICAL / WARNING / SUGGESTION

- CRITICAL: none
- WARNING: none
- SUGGESTION: 手测 footer/title 是否以后要跟 session name（本 change 未要求）

## Verdict

**Ready to archive**（未执行 archive / 人类验收；按用户要求停在验收前一步）。
