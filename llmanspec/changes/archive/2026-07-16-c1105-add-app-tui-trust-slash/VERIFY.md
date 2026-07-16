# VERIFY — c1105-add-app-tui-trust-slash

Date: 2026-07-16

## Gate

- stage: `full`
- `LLMANSPEC_BASE_REF=main llman sdd validate c1105-add-app-tui-trust-slash --strict --no-interactive` → ok
- related harness / unit + `just lint` → ok

## Spec ↔ code

| Req | Evidence | Verdict |
|-----|----------|---------|
| atm13 idle `/trust` | `effects/slash` → `persist_project_trust`；`harness_idle_slash_trust_persists_without_reload` | OK |
| atm13 busy refuse | `input_policy` + harness busy | OK |
| atm13 catalog | `product_commands` trust entry | OK |
| atm13 arg-complete | `SlashArgCompletionSource("trust")`；`c1105_trust_arg_space_shows_self_parent_deny` | OK |
| atm13 usage | `parse_trust_modes` | OK |
| atr4 persist via Driver | `InProcessDriver::persist_project_trust` + unit write under HOME | OK |
| atr4 no auto-reload | harness `reload_runtime_calls()==0`；reload 时再读 store | OK |

## CRITICAL

无。

## WARNING

无。

## SUGGESTION

- 启动 ChoicePrompt 路径仍独立；slash 不替代 gate。

## BDD

无 acceptance `feature_refs`；以 harness / unit 为准。
