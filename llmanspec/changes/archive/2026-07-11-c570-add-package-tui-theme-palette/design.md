# Design — c570-add-package-tui-theme-palette

## 目标边界

| 做 | 不做 |
|---|---|
| 两套语义色板 Dark / Light | JSON 主题、热重载、用户自定义文件 |
| paint 真彩 + `Palette` 固有方法建闭包主题 | 把 `*Theme` 改成结构体色值（组件仍收闭包） |
| demo 全量换肤 + COLORFGBG / harness feed | 产品 TUI 自动亮色；crossterm 环内盲写 OSC |

## 分层

```
┌─────────────────────────────────────────────────────────┐
│ app / agent_demo / playground                            │
│   theme_mode / data-scheme                               │
│   Palette::from(scheme).markdown_theme() → *Theme        │
└──────────────────────────▲──────────────────────────────┘
                           │ closures only
┌──────────────────────────┴──────────────────────────────┐
│ xylitol-tui components (Markdown / Diff / Choice / …)    │
└─────────────────────────────────────────────────────────┘

theme/
  paint.rs      fg_rgb / bg_rgb / word_tint（39/49）— 自由函数
  palette.rs    Palette + inherent builders + From<TerminalColorScheme>

terminal_colors.rs  ← 探测解析 SSOT
  + OSC11_BG_QUERY / CSI_COLOR_SCHEME_QUERY（常量）
  + resolve / feed helpers
  ⚠ crossterm 环内禁止盲写 query（reply = 假按键）
```

**原则**：语义 token 在 DESIGN.md；包用 **Rust 类型 + 固有方法**（非 TS Theme 服务 / JSON 市场）。组件 API 不变。

## 两套色板

| Token | Dark（`colors` / Mocha） | Light（`colors_light` / Latte） |
|---|---|---|
| on_surface | `#cdd6f4` | `#4c4f69` |
| muted | `#6c7086` | `#9ca0b0` |
| accent | `#89b4fa` | `#1e66f5` |
| user | `#cba6f7` | `#8839ef` |
| error / warning / success | Mocha | Latte |
| surface / tool-*-bg / diff-*-bg | DESIGN `colors` | DESIGN `colors_light` |

扩展：新 flavor = 新 `Palette` 常量；builders 不变。

## 工厂契约（固有方法）

- `palette.markdown_theme() -> MarkdownTheme`
- `palette.diff_theme() -> DiffTheme`
- `palette.choice_prompt_theme() -> ChoicePromptTheme`
- `Palette::from(TerminalColorScheme)`

## 探测接线（host-driven）

优先级：`explicit > OSC11 亮度 > CSI 997 > COLORFGBG > Dark`。

| 路径 | 行为 |
|---|---|
| 默认 | `theme:dark` |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | **仅 COLORFGBG**；harness `feed_terminal_color_reply` 可注入 |
| 产品 host | **不**默认打开 |

## Playground

`data-scheme=dark|light`；`sync_tokens.py` 读 `colors` + `colors_light`；右上 Dark/Light · `d`/`l` · `?scheme=light`。

## 换肤时机（demo）

`theme_mode` 变化 → 渲染路径经 `Palette::from` 重建 chrome（无缓存主题对象）。
