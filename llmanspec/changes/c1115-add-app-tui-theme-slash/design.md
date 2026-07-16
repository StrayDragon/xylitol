# Design — c1115-add-app-tui-theme-slash

## 形态（已拍板）

| 输入 | 行为 |
|---|---|
| `/theme` | 开 `EditorSlot::Themes` SelectList：`dark`、`light`（展示名可带当前标记） |
| `/theme dark` / `/theme light` | `HostSession::reload_themes` → 系统块 → `render_now` |
| `/theme toggle` / `cycle` | 相对当前 preference（无则视为 dark）翻转后 `reload_themes` |
| 其它参数 | usage / unknown 系统块；**不**改主题 |
| busy | 拒绝文案；不改主题、不开槽 |

对齐 `/model` 槽，**不**用 Plate/Settings/Choice stub。

## 应用缝

```text
PendingSlash::Theme { arg: Option<String> }
  → busy? note + return
  → None → mount_themes_picker(dark, light)
  → Some(name) → session.reload_themes(name)  // c1095
  → Ok: theme_preference 已写；系统块 `theme → {name}`
  → Err: 系统块失败；palette 不变
```

选中列表项：与 models 相同——host pending / slot Enter → `reload_themes` + 关槽。

## 与 DEMO / DESIGN

- demo `/theme` + Ctrl+P：**参考行为**；产品**不**绑 Ctrl+P theme-toggle（`theme-tokens.md`）。
- 「MUST NOT 默认开启 /theme」→ 改为：默认仍 dark；**用户 slash 可切换**；auto 仍禁。

## 补全

`SlashArgCompletionSource`：`dark`、`light`、`toggle`（`cycle` 可选别名，可不进列表）。
