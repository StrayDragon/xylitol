# Tasks — c570-add-package-tui-theme-palette

## 1. 包：paint + palette + factories

- [x] `src/theme/paint.rs`：`fg_rgb` / `bg_rgb` / 组合 tint（39/49）
- [x] `src/theme/palette.rs`：`SemanticPalette::{dark,light,for_scheme}`，token 对齐 DESIGN + Latte
- [x] `src/theme/factories.rs`：markdown / diff / choice 工厂
- [x] `lib.rs` 导出；`terminal_colors` 查询常量 + reply 识别辅助

## 2. Demo 全量换肤

- [x] `agent_demo` 用 `SemanticPalette::for_scheme(theme_mode)` 构建全部 chrome
- [x] `theme_mode` 变更时重建主题并重绘
- [x] `THEME_AUTO`：COLORFGBG + 写 OSC/CSI 查询；`feed_terminal_color_reply` harness

## 3. 文档与测试

- [x] 更新 `theme-tokens.md`（两套 + demo 全量换肤约定）
- [x] 单测：Dark≠Light；工厂可渲染；resolve + feed 换肤
- [x] `llman sdd validate c570-add-package-tui-theme-palette --strict --no-interactive`
- [x] `cargo test -p xylitol-tui`（theme / terminal_colors / demo harness 相关）
