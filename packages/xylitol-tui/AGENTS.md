# packages/xylitol-tui

本 package **稳定边界**。仓库规则与 AGENTS 写法：根 `AGENTS.md`。产品面接线：`src/app/tui/AGENTS.md`。

`xylitol-tui` 是 `pi-tui` 的 Rust 移植（差分渲染引擎 + 组件库）。**零引用**主 crate `xylitol`。

## 边界

| 是 | 不是 |
|---|---|
| `TUI` 引擎、`Component`/`Focusable`、通用组件 | agent session、slash、`XyEvent`、业务状态机 |
| 同步库；产品面 host 驱动 | 绑定 tokio / 拥有产品事件循环 |
| `lib.rs` re-export = API 边界 SSOT | 应用层 theme token / 流式业务缓冲 |

对齐源：`../pi/packages/tui`（及 kimi-code 同源）。进度/裁剪笔记：根 `_HANDOFF.md`（非规范）。

## 硬约束（不得回退）

1. 产品路径用 `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` 仅 demo。
2. 异步事件合流在 `src/app/tui/`，不进本 package。
3. `render` → `Vec<String>`（ANSI）；不引入结构化 `StyledLine` 层。
4. 主题用闭包注入；语义 token 映射在应用面。
5. **终端 I/O 优先 crossterm Command**：raw mode、bracketed paste、Kitty push/pop、`Clear`/`SetTitle`/`cursor::*`、同步输出等有库 API 就用库；仅 OSC 9;4 等库未暴露的序列才 `write_raw`。不移植 `stdin-buffer`（crossterm 已解码 `Event`）。
6. **输入硬切 = `InputEvent`（`Key`/`Paste`）**：运行时只走 crossterm `Event::Key` / `Paste` / `Resize` → `dispatch_event`；组件 `handle_input(InputEvent)`。**禁止** KeyEvent→VT→parse 运行时路径（已删除 `dispatch_input` / `key_event_to_string`）。`matches_key`/`parse_key` 仅供 `keys_test` 与配置字符串；匹配用 `matches_key_event` / `KeybindingsManager::matches_event`。测试可用 `tests/support/vt_feed.rs` 把 VT 序列译成 `InputEvent`。
7. 默认隐藏硬件光标（`show_hardware_cursor = false`）；Editor 用反色假光标。有 `CURSOR_MARKER` 时仍可相对定位 IME，但不得无条件 `show_cursor`（否则流式重绘闪烁）。
8. 有意不移植：`stdin-buffer`、`native-modifiers`、Apple/Windows 专属输入、`writeLogPath`（用 tracing）。

应用面缺底层能力时：在本包查/补，再接线——不要把通用能力做进 `src/app/tui/`。

## Specs

本包能力 specs 使用 `package-tui-*` 前缀（见根 `AGENTS.md` / `llmanspec/config.yaml`）。产品面用 `app-tui`。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 改组件 / 引擎 / 测 TUI | `test-tui-harness` skill |
| 改产品 TUI 面 | `write-tui` skill + `src/app/tui/AGENTS.md` |
| 日常验证 | `cargo test -p xylitol-tui`；`just qa`；E2E `just test-tui-e2e` |

裁剪与待补 API 随接线演进，以代码与 `_HANDOFF.md` 为准，不在本文件维护清单。

## DESIGN.md（约定，未成文）

终端域视觉/UX 规范拟用 DESIGN.md 风格（YAML tokens + 中文 rationale），token 面向 ANSI/行距/固定底栏，而非 Web px。成文时机与路径待产品面重写时再定；在此之前以本文件硬约束 + `agent_demo` 为验收参考。
