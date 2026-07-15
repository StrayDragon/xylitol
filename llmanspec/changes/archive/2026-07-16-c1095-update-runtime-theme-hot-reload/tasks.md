# Tasks — c1095-update-runtime-theme-hot-reload

## 1. Layout + UiRoot

- [x] 1.1 `LayoutTheme::from_palette` / `product_light`
- [x] 1.2 `UiRoot::apply_layout_theme` 重建 themed 子件
- [x] 1.3 单测：apply light 后 `palette()` 为 light

## 2. Discovery + host

- [x] 2.1 `app/core::discovered_theme_names`（Trust）
- [x] 2.2 `resolve_builtin_palette` + `HostSession::reload_themes`
- [x] 2.3 单测：未知名保留旧主题；dark/light 成功

## 3. 校验

- [x] 3.1 `llman sdd validate c1095-… --strict`（tasks 全勾后）
- [x] 3.2 `just qa`
