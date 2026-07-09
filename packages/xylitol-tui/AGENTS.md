# packages/xylitol-tui

本 package **稳定边界**。仓库规则与 AGENTS 写法：根 `AGENTS.md`。产品面接线：`src/app/tui/AGENTS.md`。

`xylitol-tui` 是 `pi-tui` 的 Rust 移植（差分渲染引擎 + 组件库）。**零引用**主 crate `xylitol`。

## 边界

| 是 | 不是 |
|---|---|
| `TUI` 引擎、`Component`/`Focusable`、通用组件 | agent session、slash、`XyEvent`、业务状态机 |
| 同步库；产品面 host 驱动 | 绑定 tokio / 拥有产品事件循环 |
| `lib.rs` re-export = API 边界 SSOT | 应用层 theme token / 流式业务缓冲 |

对齐源：`../pi/packages/pi-tui`、`../kimi-code/packages/pi-tui`。进度/裁剪笔记：根 `_HANDOFF.md`（非规范）。

## 硬约束（不得回退）

1. 产品路径用 `dispatch_input` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` 仅 demo。
2. 异步事件合流在 `src/app/tui/`，不进本 package。
3. `render` → `Vec<String>`（ANSI）；不引入结构化 `StyledLine` 层。
4. 主题用闭包注入；语义 token 映射在应用面。
5. 输入/Resize/Paste 基线靠 `crossterm`；终端增强集中在 `terminal.rs`。
6. 有意不移植：`stdin-buffer`、`native-modifiers`、Apple/Windows 专属输入、`writeLogPath`（用 tracing）。

应用面缺底层能力时：在本包查/补，再接线——不要把通用能力做进 `src/app/tui/`。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 改组件 / 引擎 / 测 TUI | `test-tui-harness` skill |
| 改产品 TUI 面 | `write-tui` skill + `src/app/tui/AGENTS.md` |
| 日常验证 | `cargo test -p xylitol-tui`；`just qa`；E2E `just test-tui-e2e` |

裁剪与待补 API 随接线演进，以代码与 `_HANDOFF.md` 为准，不在本文件维护清单。
