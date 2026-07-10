# packages/xylitol-tui

本 package **稳定边界**。仓库规则与 AGENTS 写法：根 `AGENTS.md`。产品面接线：`src/app/tui/AGENTS.md`。产品视觉/UX：`src/app/tui/DESIGN.md`。

`xylitol-tui` 是以 `pi-tui` 为参考的 Rust **引擎 + 通用组件库**，不是 1:1 完整移植。**零引用**主 crate `xylitol`。

## 边界

| 是 | 不是 |
|---|---|
| `TUI` 引擎、`Component`/`Focusable`、通用组件 | agent session、slash、`XyEvent`、业务状态机 |
| 同步库；产品面 host 驱动 | 绑定 tokio / 拥有产品事件循环 |
| `lib.rs` re-export = API 边界 SSOT | 应用层 theme token / 流式业务缓冲 / 产品 layout |

对齐源（行为参考，非逐文件镜像）：`../pi/packages/tui`。
**刻意差异台账（整合时防覆盖）**：本包 [`PI_DELTAS.md`](PI_DELTAS.md)。进度笔记：根 `_HANDOFF.md`（非规范）。

## 与 pi-tui 的刻意差异（不得回退成「完整 port」）

目标已从「完整移植 pi-tui」收敛为：**差分渲染 + 可组合组件 + crossterm 原生输入**，产品壳与视觉在 `src/app/tui/`。

### 底层 / 输入

| 主题 | pi-tui | xylitol-tui |
|---|---|---|
| 终端 I/O | 自管 VT / raw 序列较多 | **优先 crossterm Command**（raw、paste、Kitty push/pop、`Clear`/`SetTitle`/`cursor::*`、同步输出）；仅 OSC 9;4 等库未暴露的才 `write_raw` |
| stdin | 自研 `stdin-buffer` | **不移植**；crossterm 已解码 `Event` |
| 按键模型 | VT 字符串 / 自解析为主 | **硬切 `InputEvent::{Key,Paste}`**；禁止 KeyEvent→VT→parse 运行时路径 |
| 键匹配 | 字符串 `matchesKey` 等 | 运行时 `matches_key_event` / `KeybindingsManager::matches_event`；`matches_key`/`parse_key` 仅测试与配置字符串 |
| 平台专属输入 | `native-modifiers`、Apple/Windows 路径 | **不移植** |
| 调试写盘 | `writeLogPath` | **不移植**（用 tracing / 应用面日志） |

### 引擎 / API 形状

| 主题 | pi-tui | xylitol-tui |
|---|---|---|
| 根类型 | `TUI extends Container` | `TUI` 持有根 `components`；另提供独立 `Container` 组件（组合，非继承） |
| Overlay | `OverlayHandle` + 完整 focus-restore 状态机 | `OverlayHandle`（id 世代）+ 最小 hide/focus/unfocus；**完整 eligible/blocked restore 不移植**（见归档 c445 `future.md`） |
| 事件循环 | 库内 `start` 常见 | 产品路径 **host 驱动** `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` **仅 demo** |
| 硬件光标 | 可开 | 默认 **隐藏**；Editor 用反色假光标（防流式闪烁） |
| 渲染输出 | `string[]` ANSI | 同：`Vec<String>`；**不**引入结构化 `StyledLine` |
| Editor 补全 | provider + 引擎内 `/` 特判较多 | **`CompletionSource` 注册表**；引擎只管 popup；`/` `@` 等为可插拔 Source（见 `completion.rs`） |

### 有意不移植 / 已裁剪的模块

- `stdin-buffer`、`native-modifiers`、Apple/Windows 专属输入、`writeLogPath`
- **`Image` 组件**与 Kitty/iTerm **完整 encode 路径**（c445 裁剪）；保留 `is_image_line`（宽度豁免）与 `hyperlink`
- pi coding-agent **产品壳**（transcript/slash/session UI）→ 在 `src/app/tui/`，不进本包
- 完整 overlay focus-restore 状态机（延后，非本包硬目标）

缺能力时：**先在本包补通用能力，再由应用面接线**——不要把准通用实现塞进 `src/app/tui/`。

## 硬约束（不得回退）

1. 产品路径用 `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` 仅 demo。
2. 异步事件合流在 `src/app/tui/`，不进本 package。
3. `render` → `Vec<String>`（ANSI）；不引入结构化 `StyledLine` 层。
4. 主题用闭包注入；语义 token 映射在应用面（`src/app/tui/DESIGN.md`）。
5. 终端 I/O / 输入硬切：见上表「底层 / 输入」。
6. 默认隐藏硬件光标；有 `CURSOR_MARKER` 时可相对定位 IME，但不得无条件 `show_cursor`。

## Specs

本包能力 specs 使用 `package-tui-*` 前缀（见根 `AGENTS.md` / `llmanspec/config.yaml`）。产品面用 `app-tui-*`（不再堆进单体 `app-tui`）。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 改组件 / 引擎 / 测 TUI | `test-tui-harness` skill |
| 扩展 Editor 补全触发（`/` `@` `$` `^`…） | `CompletionSource` + `set_completion_sources`（`src/completion.rs`）；勿在 `editor.rs` 硬编码触发符 |
| 对照 / 合并 pi-tui 行为 | 先读 [`PI_DELTAS.md`](PI_DELTAS.md)；不得静默回退表中决议 |
| 改产品 TUI 面 / UX | `write-tui` skill + `src/app/tui/AGENTS.md` + `src/app/tui/DESIGN.md` |
| 日常验证 | `cargo test -p xylitol-tui`；`just qa`；E2E `just test-tui-e2e` |

裁剪与待补 API 随接线演进，以代码与 `_HANDOFF.md` 为准，不在本文件维护进度清单。
