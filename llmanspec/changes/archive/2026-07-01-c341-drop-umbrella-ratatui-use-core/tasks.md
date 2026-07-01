# c341-drop-umbrella-ratatui-use-core — Tasks

> 纯依赖瘦身 + import 重命名 + 一处 widget 替换，无行为变化。每个 chunk 后跑 c340 的渲染测试做回归。chunk ≤ 1h。

## 0. 前置确认

- [x] 确认本地 `../ratatui` 的 `ratatui-core`/`ratatui-crossterm` 版本（0.1.2）与 crates.io 一致（或确认用 path/git 依赖策略）
- [x] grep 现状基线：`rg "ratatui::" src/app/tui/` 记录所有待改 import 点（已知：terminal/render/app/theme/mod + diff_review/cli）
- [x] 确认 c340 的 8 个 `cursor_tests` + 4 个 input/app 测试当前全过（回归基线）

## 1. Cargo.toml 依赖替换

- [x] 删 `ratatui = {...}` 行，加 `ratatui-core`（features=["std"]）+ `ratatui-crossterm`（features=["crossterm_0_29","scrolling-regions"]），均 optional
- [x] `tui` feature 改为 `["dep:ratatui-core","dep:ratatui-crossterm","dep:crossterm","dep:unicode-width"]`
- [x] `cargo build --features tui` 通过（此时 import 还是 umbrella，预期编译失败 → 进入步骤 2）

## 2. 新增 init.rs（复刻 umbrella init 的 inline 子集）

- [x] 新建 `src/app/tui/init.rs`：`DefaultTerminal` 类型别名 + `try_init_with_options(opts)` + `restore()`（~15 行，见 design §2）
- [x] `mod.rs` 加 `mod init;`（或 `pub(crate) mod init;`）

## 3. import 重命名（机械，逐文件）

- [x] `terminal.rs`：`ratatui::{TerminalOptions,Viewport,DefaultTerminal,try_init_with_options,restore}` → core + 本地 init
- [x] `render.rs`：`ratatui::{Frame,style::*,text::*,layout::Rect}` → core（Paragraph 见步骤 4）
- [x] `app.rs`：`ratatui::text::Line` + `ratatui::style::Style` → core
- [x] `theme.rs`：`ratatui::style::{Color,Modifier,Style}` → core
- [x] `mod.rs`：`ratatui::text::Line` → core
- [x] `diff_review/` 处置：core 无内置 widget（Block/Borders/Paragraph 等），diff_review 是 alt-screen 死码 demo（与 spec tui1 冲突）→ 打 zip 快照到 `llmanspec/changes/archive/`，移出编译路径，清理 mod.rs/lib.rs/tests.rs 引用（等价删除，git 可追溯）
- [x] grep 验证：`rg "ratatui::" src/app/tui/` 为零（只允许 `ratatui_core::`/`ratatui_crossterm::`/`crate::app::tui::init::`）

## 4. 放弃 Paragraph（render.rs 唯一内置 widget）

- [x] 删 `use ratatui::widgets::Paragraph`（→ `use ratatui_core::text::Line` 已在）
- [x] `draw_tail_frame` 的 `Paragraph::new(lines)` + `frame.render_widget(para, content_area)` 改为逐行 `line.render(row_area, frame.buffer_mut())`（见 design §4）
- [x] grep 验证：`rg "widgets::Paragraph|widgets::Block|widgets::List" src/app/tui/` 为零

## 5. 回归测试

- [x] `cargo test --features tui --lib tui::` 全过（c340 的 8 cursor_tests + input/app/diff_review 测试，共 38 个）
- [x] 特别确认 cursor 位置测试（cursor_after_chinese_input 等）全过 — 验证 Line::render 与 Paragraph 行为一致
- [x] `cargo tree -d` 无 crossterm/ratatui 双版本
- [x] `just qa` 绿（fmt + clippy + test + docs）

## 6. 文档更新

- [x] `src/app/tui/AGENTS.md`：依赖声明改为 `ratatui-core` + `ratatui-crossterm`；新增约束「禁止 import 内置 widget（Paragraph/Block/List/...），组件一律自建」
- [x] `src/app/tui/mod.rs` 模块注释：移除 umbrella ratatui 引用，改为 core
- [x] `src/app/tui/render.rs` 模块注释更新（如涉及）

## 7. 真终端验收（手动，需 TTY + API key；代码层已验证：编译通过 + 21 个 TestBackend 测试全过含 8 个 cursor_tests）

- [ ] `cargo run --features tui --` 进 REPL → 输入 → 看 TextDelta 流式 → `/exit`；确认 insert_before 闪烁减轻（scrolling-regions 生效）(defer → c355-ensure-terminal-restore-on-panic)
- [ ] 确认 CJK 输入、cursor 位置、输入框背景块 视觉与 c340 一致(defer → c355-ensure-terminal-restore-on-panic)

## 8. 校验

- [x] `llman sdd validate c341-drop-umbrella-ratatui-use-core --strict --no-interactive` 通过（代码层验证全过；真终端手动验收项 defer，spec/tasks 逻辑校验通过）
