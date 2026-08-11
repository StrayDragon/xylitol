# Research: crossterm 0.29.0 鼠标 API（一手）

> Change: `c2020-add-package-tui-mouse-input`
> 锁定版本：本仓 `Cargo.lock` → `crossterm` **0.29.0**（checksum `d8b9f2e4…`）；`Cargo.toml` / `packages/xylitol-tui` 声明 `0.29`。
> 一手范围：docs.rs `crossterm/0.29.0` + crates.io 解压源码（vcs sha `36d95b26a26e64b0f8c12edfe11f410a6d56a812`）。
> 非 live specs；不改 proposal。

## 1. Enable / Disable

鼠标**默认不捕获**。模块文档写明须用 `EnableMouseCapture`（焦点则用 `EnableFocusChange`）开启；可用 `read` / `poll`（或 `EventStream`，勿与 `read`/`poll` 混用或跨线程）收取事件。
来源：[docs.rs …/event/index.html](https://docs.rs/crossterm/0.29.0/crossterm/event/index.html)；源码 `src/event.rs` 模块注释 L20–24。

用法形态：`execute!(stdout, EnableMouseCapture)?` / `DisableMouseCapture`；二者实现 `Command`。文档描述仅一句：启用/禁用 mouse event capturing，事件经 `read`/`poll` 捕获。
来源：[EnableMouseCapture](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.EnableMouseCapture.html)、[DisableMouseCapture](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.DisableMouseCapture.html)。

Unix ANSI 序列（`write_ansi`），源码注释逐项说明：

| Enable (`?…h`) | 注释语义 |
|---|---|
| `1000` | Normal tracking：按键按下/释放报 X/Y |
| `1002` | Button-event：拖拽时的 button motion |
| `1003` | Any-event：**所有** motion（含无按键移动） |
| `1015` | RXVT mouse：坐标可 >223 |
| `1006` | SGR mouse：坐标可 >223，优先于 RXVT |

Disable 按逆序关：`1006` → `1015` → `1003` → `1002` → `1000`（`l`）。Windows 走 `execute_winapi`，且 `is_ansi_code_supported() == false`。
来源：`src/event.rs` L313–375（crates.io 0.29.0）。

## 2. `Event::Mouse` 与 `MouseEventKind`

`Event` 变体（同级还有 `Key` / `Paste` / `Resize` / focus）：

```text
FocusGained | FocusLost | Key(KeyEvent) | Mouse(MouseEvent)
| Paste(String)  // feature bracketed-paste
| Resize(u16, u16)
```

来源：[Event](https://docs.rs/crossterm/0.29.0/crossterm/event/enum.Event.html)；`src/event.rs` L550–566。

`MouseEvent { kind, column, row, modifiers }`。平台注记：部分终端对 `Up`/`Drag` 不报真实按钮 → 回落 `MouseButton::Left`；修饰键组合不全（例：macOS 把 Ctrl+左键报成右键）。
来源：[MouseEvent](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.MouseEvent.html)；`src/event.rs` L760–786。

`MouseEventKind`：`Down(btn)` / `Up(btn)` / `Drag(btn)` / `Moved` / `ScrollDown` / `ScrollUp` / `ScrollLeft` / `ScrollRight`。
`MouseButton`：`Left` / `Right` / `Middle`。
来源：[MouseEventKind](https://docs.rs/crossterm/0.29.0/crossterm/event/enum.MouseEventKind.html)；`src/event.rs` L800–817、L823–829。

Unix SGR 解析：`CSI < Cb ; Cx ; Cy M|m`（`parse_csi_sgr_mouse`）；`parse_cb` 从 Cb 解出 kind，并从 bit 抽出 `SHIFT` / `ALT` / `CONTROL`。
来源：`src/event/sys/unix/parse.rs` L718–809。

## 3. 与 bracketed paste / Kitty keyboard 共存

- **独立 Command、独立 DEC/协议**：paste = `?2004h/l`（`EnableBracketedPaste`）；Kitty = `CSI >{flags}u` / `CSI <1u`（`PushKeyboardEnhancementFlags` / `PopKeyboardEnhancementFlags`）；mouse = 上表 `1000` 等。互不共用同一开关位。
  来源：`src/event.rs` L413–527；[PushKeyboardEnhancementFlags](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.PushKeyboardEnhancementFlags.html)。
- **同一事件泵**：`Event` 用不同变体分流；`Paste` 仅在 bracketed paste 已启用时发出。
  来源：`src/event.rs` L559–561；模块 docs 示例把 `EnableBracketedPaste` + `EnableFocusChange` + `EnableMouseCapture` 同 `execute!` 开启，退出时成对 Disable。
  来源：[event 模块 Examples](https://docs.rs/crossterm/0.29.0/crossterm/event/index.html)。
- **官方 example 进一步并列 Kitty**：`examples/event-read.rs` 在 raw mode 下可选 `PushKeyboardEnhancementFlags(…)`，再 `EnableBracketedPaste` + `EnableFocusChange` + `EnableMouseCapture`；退出先 Pop（若推过），再 Disable paste/focus/mouse。未声明互斥或冲突。
  来源：crates.io 0.29.0 `examples/event-read.rs` L68–111。
- **注意点（仍属一手可证）**：键盘事件文档要求 raw mode；mouse/focus 另须 Enable。`read`/`poll` 与 `EventStream` 不可混用。Kitty 增强只影响键事件信息量，文档未称会改变 mouse 解析。
  来源：`src/event.rs` L12–24、L455–491。

## 4. Terminal selection（选区）副作用 — 库内空白

在 **0.29.0** 的 docs.rs 鼠标相关页、`src/event.rs` Enable/Disable 注释、README 鼠标条目、以及全文检索 `selection`（除 OSC52 `ClipboardSelection` 与 clipboard 模块）中：**没有**「启用 mouse capture 后终端拖选/复制失效」「Shift 透传选区」之类记载。

一手可陈述的仅有：`EnableMouseCapture` **固定**打开 `1003` any-event tracking（全部 motion）与 `1002`/`1000`，因此应用侧会收到 `Moved`/`Drag` 流；**选区 UX 副作用不在 crossterm 契约内**，须另查终端仿真器 / xterm 私有模式文档（超出本笔记一手范围）。

## 5. 对本 change 的可引用事实摘要

1. 成对 `EnableMouseCapture` / `DisableMouseCapture`；默认关。
2. 事件模型已有完整 `Event::Mouse` + `MouseEventKind`（含 scroll / moved）。
3. Enable 即开 `1003` → 默认会有 move 洪水；过滤须在应用层。
4. 与 paste / Kitty 在官方示例中可同开；清理应成对、顺序可参考 `event-read.rs`。
5. 选区 tradeoff：**crossterm 未文档化**；不可把「库保证 Shift 选区」写进依赖库行为。
