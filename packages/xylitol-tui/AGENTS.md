# packages/xylitol-tui

本文件只放本 package **专属**规则。仓库级规则见根 `AGENTS.md`；应用面接线见 `src/app/tui/AGENTS.md`。

`packages/xylitol-tui` 是 `pi-tui` 的 Rust 移植（差分渲染引擎 + 组件库）。**零引用** `xylitol` 主 crate，不夹带应用层耦合。目标是作为 `src/app/tui/` 的渲染与组件底座。

> **改本 package？** 先读本文件的边界与硬约束，再对照 `_HANDOFF.md` 的模块状态。应用面改造走 `write-tui` skill，不要把业务逻辑写进本 package。

## 定位与边界

- **是**：通用终端 UI 库（`TUI` 引擎、`Component`/`Focusable`、Editor/Input/Markdown/SelectList 等）。
- **不是**：agent session、slash 命令、`XyEvent` 翻译、多 session / 业务状态机。
- **对齐源**：`../pi/packages/pi-tui` 与 `../kimi-code/packages/pi-tui`。模块对照与移植进度见仓库根 `_HANDOFF.md`。
- **`lib.rs` re-export** 对齐 pi 的 `index.ts`（本 package 的 API 边界 SSOT）。

## 架构决议（已定）

下列是 review 后已确认的边界，后续改动不得回退：

1. **引擎同步、host 驱动**：本 package 保持同步库。产品面用 `dispatch_input` / `request_render` / `try_render` / `idle_tick` 驱动；`TUI::start()` 仅给 demo / playground。
2. **异步事件循环属于应用面**：键盘、agent 事件、tick 的合流在 `src/app/tui/`（App Shell），不把 tokio 绑进本 package。
3. **样式模型**：组件 `render` 返回 `Vec<String>`（原始 ANSI）。不在本 package 引入 `StyledLine`/`Span` 结构化层。
4. **主题模型**：组件接收闭包 theme（如 `Box<dyn Fn(&str) -> String>`）。共享语义 token 映射放应用面 `theme.rs`，不放本 package。
5. **流式缓冲**：fence-aware / 双缓冲等业务流式逻辑放应用面，本 package 不提供 `StreamingText` 业务 widget。

## 跳过项（有意不移植）

| pi 模块 / 能力 | 原因 |
|---|---|
| `stdin-buffer.ts` | crossterm 已处理 escape 拼接 / bracketed paste / Kitty / OSC |
| `native-modifiers.ts` | macOS 原生二进制，不可移植 |
| `index.ts` | 等价本 package `lib.rs` |
| Windows VT / Apple Terminal 专属输入 | crossterm 覆盖；Linux 优先 |
| `writeLogPath` | 用 `tracing` |

## 裁剪策略

主动移植已完成；后续按需裁剪，不继续为对齐而扩张。

**立即可裁（应用面接线前）**

- `components/image`：应用层不用终端图片时整模块可删。
- `terminal_image`：保留 `is_image_line` / `TerminalCapabilities` 等引擎所需最小面，其余搁置。
- `autocomplete_fd`：按需 feature 门控（如 `fd-path`），默认不强迫依赖。

**接线后再裁**

- `terminal_colors`、`autocomplete` 的未用 provider / 解析颗粒度。

**必须保留的核心链**

`tui` → `terminal` / `keys` / `utils` / `keybindings` / `paste_burst` → `components/{editor,input,markdown,select_list,settings_list,text,panel,loader,…}` → `autocomplete` / `fuzzy` / `undo_stack` / `kill_ring` / `word_navigation` / `clock`。

## 待补 API（应用面接线前）

| API | 优先级 | 说明 |
|---|---|---|
| `Container` | 高 | 可嵌套组件树 |
| `OverlayHandle` | 高 | hide / focus / unfocus / setHidden |
| `InputListener` | 中 | 输入到达焦点组件前的拦截管线 |

暂缓：overlay focus restore 状态机、color scheme 查询/通知、OSC 11 自动接线、onDebug、crash 文件（用 tracing）。

## 终端支持矩阵

- **主支持面**：Linux / xterm-compatible —— `foot`、`wezterm`、`ghostty`、`alacritty`、`kitty`、`tmux`。
- **基线**：输入 / Resize / Paste 默认依赖 `crossterm`，不要在本 package 重复实现基础终端抽象。
- **保留增强**：Kitty keyboard / `modifyOtherKeys` fallback / bracketed paste / OSC 标题与进度；集中在 `terminal.rs`，不向组件层泄漏。
- **规则**：新增终端兼容代码前，先证明 `crossterm` 不能覆盖；example 缺陷先查布局/宽度预算，勿误判为协议缺陷。

## 测试约定（c405 五层）

本 package 承载第 1–4 层（in-process）；第 5 层在 workspace 顶层 `tests/tui_e2e/`。

| 层 | 落点 | 用于 |
|---|---|---|
| 1. 按键序列→状态 | `tests/support/mod.rs::TuiTestHarness` + `tests/harness_test.rs` | 组件交互 |
| 2. insta snapshot | `tests/snapshot_test.rs` + `tests/snapshots/` | 整屏渲染回归 |
| 3. 时序 | `src/clock.rs` + `#[tokio::test(start_paused)]` | paste-burst / debounce |
| 4. proptest | `tests/property_test.rs` | Editor 不变量 |
| 5. E2E | `tests/tui_e2e/`（workspace） | 真 PTY / tmux；`#[ignore]`，经 `just test-tui-e2e` |

### 新增测试规则

- 组件交互 → 第 1 层 harness。
- 渲染回归 → 第 2 层 snapshot；`INSTA_UPDATE=always` 接受后人工复核。
- 时序逻辑 → 第 3 层；**禁止 `thread::sleep`**。
- Editor 边界 → 第 4 层随机按键 + 不变量。
- snapshot 文件进版本控制；prek trailing-whitespace 会删 `.snap` 尾空格——空行不得留尾空格。

## 编码约定

- 不过度封装；一两行逻辑直接内联。
- 宽度不变量：`visible_width(line) > width` → `RenderError`（调用方决定是否降级，本库不静默截断除非另有决议）。
- 新模块落点先查是否已有归属；API 变更同步 `lib.rs` re-export。
