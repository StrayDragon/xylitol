# VERIFY — c1120-add-app-tui-reload-slash

Date: 2026-07-16

## Gate

- stage: `full`
- `llman sdd validate c1120-add-app-tui-reload-slash --strict --no-interactive` → ok
- `just qa` → ok（apply 阶段已绿）

## Spec ↔ code

| Req | Evidence | Verdict |
|-----|----------|---------|
| atm12 idle `/reload` | `effects/slash.rs` `handle_reload`；harness `harness_idle_slash_reload_keeps_history_and_calls_runtime` | OK |
| atm12 busy refuse | `input_policy` busy Enter + `PendingSlash::Reload` arm；`harness_busy_slash_reload_refused` | OK |
| atm12 catalog | `product_commands` + `parse_slash_command`；SSOT catalog test | OK |
| ath20 foundations via Driver seam | `Driver::reload_runtime` on `InProcessDriver`；host keybindings/themes；`set_dollar_skill_catalog` | OK |
| ath20 partial success | `reload_runtime` 逐步 `ReloadStepReport`，单步失败不中断 | OK |
| 历史不变 | harness 断言 entries 仅 +1 system note | OK |

## CRITICAL

无。

## WARNING

- ath20「磁盘 skills 变更后 catalog 刷新」在 ScriptedDriver 下以 `reload_runtime_calls` + catalog setter 覆盖；真磁盘 skills 变更更偏手工/后续 c1200 波次。

## SUGGESTION

- `/reload` 进行中无 spinner（已记 deferred `c1205`）。

## BDD

本 change delta 无 `feature_refs` acceptance BDD；以 harness 单测为准。
