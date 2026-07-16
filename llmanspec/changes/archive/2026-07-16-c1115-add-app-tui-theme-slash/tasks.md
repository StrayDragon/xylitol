# Tasks — c1115-add-app-tui-theme-slash

## 1. Slash + 槽

- [x] 1.1 `PendingSlash` / `parse_slash_command`：`/theme` · 参 · usage
- [x] 1.2 catalog + `SlashArgCompletionSource`（dark/light/toggle）
- [x] 1.3 `EditorSlot::Themes` + UiRoot mount/select/Esc（对齐 Models）
- [x] 1.4 `effects/slash`：busy 闸；无参开槽；有参/`toggle` → `reload_themes`

## 2. 文档与测

- [x] 2.1 更新 `design/theme-tokens.md`（用户 slash 允许；禁默认 auto / Ctrl+P）
- [x] 2.2 harness：light ok、坏名保留、busy 拒绝、无参开槽、catalog
- [x] 2.3 `LLMANSPEC_BASE_REF=main llman sdd validate c1115-… --strict --no-interactive`
- [x] 2.4 `just lint` + 相关测
