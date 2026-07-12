# Tasks — c475-add-app-tui-chrome

- [x] 1. 落地 `glyphs` + `theme`（Palette::dark → EditorTheme / muted paint）
- [x] 2. `UiRoot`：独立 status 槽；footer=`cwd · model`（+ 可选 q:）；去键墙；border muted
- [x] 3. host / `run` 注入 cwd + model；scrollback 前缀走 glyph
- [x] 4. 单测：idle 0 status 行、busy status 不进 footer、footer 无键墙、ascii glyph
- [x] 5. `llman sdd validate c475-add-app-tui-chrome --strict --no-interactive`
- [x] 6. `just qa`（或至少 app tui 测试 + lint）
