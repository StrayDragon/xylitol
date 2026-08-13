# Scene 与社区工具（npm / crates）

> c2200 补充。结论：**不引入外引擎 Storybook**；学其「命名隔离态」，渲染必须走 xylitol 产品根。

## 产品链路（本仓已有）

`/debug activity-fold-live`（`effects/debug_activity_fold.rs`）已经：

```text
clone UiModel → replay tape → UiRoot::apply_ui_model → root.render → 断言
```

这就是 Scene。再造一套 paint = 第二 SSOT。正确迭代 = **扩 debug fixture / live_tape / HostSession harness**，不是新名词框架。

分层（别抽象过头）：

| 层 | 测什么 | 现成 |
|---|---|---|
| 纯函数 | `count_*` / `format_*` | `summary.rs` 单测 |
| 产品根 | 真 `UiRoot`/`HostSession` paint | `live_tape`、`harness.rs`、`new_product_ui` |
| 真终端 | PTY/tmux | `tests/tui_e2e/`（稀疏） |

「Scene」= 第二层的 **命名夹具**，不是新渲染器。

## npm

| 包 | 做什么 | 与 xylitol |
|---|---|---|
| [ink-storybook](https://github.com/expelledboy/ink-storybook) | TTY 里浏览 `.story.tsx` | Ink/React；引擎不同 |
| [ink-testing-library](https://www.npmjs.com/package/ink-testing-library) | `render` → `lastFrame()` / `frames[]` / fake timers | **帧序列 API 可抄**；实现用我们的 `render`+Clock |
| [Kaleidoscope](https://github.com/RogerSquare/kaleidoscope) | 每 demo 一个 Ink 组件，真 TTY | 学「隔离 demo」；勿抄 React |
| InkUI | shadcn 式拷贝组件 | 我们已有 `xylitol-tui` 原子 |

## crates.io

| crate | 做什么 | 与 xylitol |
|---|---|---|
| [tui-pantry](https://docs.taho.is/tui-pantry) | ratatui Storybook：`Ingredient` + `cargo pantry` | **不要依赖**：ratatui `Rect`/`Widget`，我们是 `Component`→`Vec<String>` ANSI。可抄四档：Widgets / Panes / Views / Styles |
| ratatui `TestBackend` + [insta](https://ratatui.rs/recipes/testing/snapshots/) | 内存 buffer 黄金屏 | 我们已有 VirtualTerminal + insta |
| [ratatui-testlib](https://crates.io/crates/ratatui-testlib) | PTY | 我们已有 portable-pty 层 5 |
| tui-realm | Elm/React 式框架 | 换引擎，否 |

Pantry 文档要点（[Why](https://docs.taho.is/tui-pantry)）：Figma 格子是方的、终端约 1:2；溢出在 TUI **静默裁切**。所以预览必须是真格子，不是 HTML。

## 建议

1. 不加 npm/ratatui 依赖。
2. 人看的目录：`just` 挂 **产品** `HostSession` + 选 debug fixture（扩 `debug_fixtures/catalog.rs`）。
3. 测：`ink-testing-library` 的 `frames[]` = 我们对同一 `HostSession` 逐步 `idle_tick`/`render`。
4. 语义 dump（L3/L2/L1）叠在产品 render 之上，解决截图看不懂和弦。
