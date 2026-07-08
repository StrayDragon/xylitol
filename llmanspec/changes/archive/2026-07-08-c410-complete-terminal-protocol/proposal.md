---
change_id: c410-complete-terminal-protocol
title: "complete terminal protocol — Kitty 键盘协议探测 + modifyOtherKeys + OSC 标题/进度（对齐 pi terminal.ts）"
status: draft
priority: 410
depends_on: []
author: agent
---

# c410-complete-terminal-protocol

## Why

`packages/xylitol-tui/src/terminal.rs` 当前仅 85 行（crossterm 薄封装），而 pi `terminal.ts` 是 531 行。这是 _HANDOFF §已知差距标注的「中等风险」缺口——`keys.rs` 已经有完整的 `set_kitty_protocol_active` + `is_kitty_protocol_active` + 按 kitty 状态分支解析的代码，**但没有任何代码去查询和激活 kitty 协议**。结果 `is_kitty_protocol_active()` 永远返回 false，keys.rs 永远走非-kitty 分支，Ctrl+Shift 组合键等增强键盘事件的解析能力被闲置。

按 _HANDOFF §二 阶段 6.1（路线 B：先补齐 package），terminal.rs 必须先扩到完整，因为它是 paste-burst（6.2）/ autocomplete（6.3）/ editor（6.4）的输入基础——后续移植都假设 kitty 协议状态被正确探测。

### 证据：pi terminal.ts 做了什么（xy 缺的）

| pi 能力 | pi 行数 | xy 现状 | 影响 |
|---|---|---|---|
| Kitty 键盘协议查询 + 激活 | ~100 行（queryAndEnableKittyProtocol + 状态机）| **零** | keys.rs 的 kitty 分支永远不激活 |
| modifyOtherKeys 回退 | ~10 行 | **零** | Ctrl+Shift 等组合键在非-kitty 终端也拿不到 |
| 协议响应解析（CSI ?Nu / DA） | ~50 行 | **零** | 无法判断终端是否支持 kitty |
| OSC 0/2 标题 | 3 行 | **零** | 终端标题不被设置 |
| OSC 9;4 进度指示 | ~15 行 | **零** | 任务栏进度不被设置 |
| drainInput（退出前排空）| ~35 行 | **零** | 慢 SSH 上 kitty key release 泄漏到父 shell |
| moveBy / setProgress | ~10 行 | **零** | API 不完整 |

### 证据：架构差异的解法（用户已定）

pi 的 terminal **拥有原始 stdin 字节流**（经 StdinBuffer 拦截 Kitty 响应再转发）。xy 用 crossterm `event::read()` 已经把字节流解析成 `KeyEvent`——**xy 不读原始字节**。

用户决策（路线 A：最小改动）：保留 crossterm 输入层，terminal 在 `start()` 时发送 Kitty 查询序列，用一个**探测窗口**（crossterm event::poll + event::read 抓 CSI 响应）判断终端是否支持 kitty，支持则 `set_kitty_protocol_active(true)`。Kitty 激活是「尽力而为」——依赖终端在窗口内响应。

## What Changes

### 新增：Kitty 键盘协议探测（路线 A）

1. `terminal.rs` 加 `probe_kitty_protocol()`：发送 `CSI >7u CSI ?u CSI c`（pi 的 `KITTY_KEYBOARD_PROTOCOL_QUERY`），然后 poll ~50ms 试图读 `Event::Key` 解析出的 CSI ?Nu 响应（crossterm 把协议响应也当 Key 事件）。
2. 响应解析：`parse_kitty_flags(CSI ?Nu)` → flags 非 0 则 `set_kitty_protocol_active(true)` + disable modifyOtherKeys；flags=0 或超时/收到 DA → fallback `enable_modify_other_keys()`。
3. 探测在 `CrosstermTerminal::start()`（新增方法，tui.rs 的 `start_impl` 调用）时执行一次。

### 新增：modifyOtherKeys 协议

`enable_modify_other_keys()` 发 `CSI >4;2m`，`disable_modify_other_keys()` 发 `CSI >4;0m`。状态机：kitty 激活则禁用 modifyOtherKeys；kitty 未激活则启用。

### 新增：OSC 标题 + 进度

- `set_title(title)` 发 `OSC 0;title BEL`
- `set_progress(active: bool)` 发 `OSC 9;4;3` / `OSC 9;4;0`（pi 用 keepalive interval 1s，xy 简化为单次发送，不做 keepalive——TUI 退出时清）

### 新增：drainInput

退出前（`stop()`）短时排空 stdin（pi 的 drainInput）：disable kitty 协议（`CSI <u`）后 poll ~100ms 读掉残留 key release 事件，防慢 SSH 泄漏。

### Terminal trait 扩展

trait 加 `set_title` / `set_progress` / `move_by` / `start()` / `stop()`（当前只有 CrosstermTerminal 的 new，trait 没有生命周期方法）。`VirtualTerminal`（test harness）对应实现为 no-op 或记录调用。

### 测试（c405 第 1+5a 层）

- **第 1 层**（单测）：`parse_kitty_flags` 纯函数测（CSI ?7u → flags=7；CSI ?0u → flags=0；非匹配 → None）
- **第 5a 层**（E2E）：portable-pty spawn demo，在真终端下验证 kitty 协议查询序列被发出（读字节流断言含 `CSI >7u`）+ 非 kitty 终端（如默认 PTY）fallback 到 modifyOtherKeys
- 探测窗口逻辑（poll/timeout）用 c405 第 3 层 Clock 注入（避免真实 sleep）

## Capabilities

- `terminal-protocol`（新建：终端协议协商规范）

## Impact

- `packages/xylitol-tui/src/terminal.rs`：85 → ~250 行（加协议探测/标题/进度/drain/trait 扩展）
- `packages/xylitol-tui/src/lib.rs`：导出新公开 API（set_title/set_progress/parse_kitty_flags）
- `packages/xylitol-tui/tests/`：加 terminal 单测（第 1 层）
- `tests/tui_e2e/pty.rs`：加 kitty 协议查询 E2E（第 5a 层）
- `llmanspec/specs/terminal-protocol/spec.toon`（新建）

## Non-goals

- **不重写输入模型**（保留 crossterm event::read，不自管原始字节流——那是路线 B 的范围）
- **不移植 StdinBuffer 的 Kitty 响应拦截状态机**（pi 的 keyboardProtocolNegotiationBuffer 那套）——路线 A 下 crossterm 替我们做输入解析，探测窗口只在 start 时跑一次
- **不移植 Apple Terminal 归一化**（`normalizeAppleTerminalInput`，macOS 原生，xy 跑 Linux/crossterm 不需要）
- **不移植 Windows VT input**（pi 的 enableWindowsVTInput，crossterm 已处理跨平台）
- **不移植 write log**（pi 的 PI_TUI_WRITE_LOG 调试日志，xy 用 tracing）

## 风险

| 风险 | 缓解 |
|---|---|
| crossterm 可能把 CSI 响应当 Key 事件解析错 | 探测窗口内对无法识别的事件忽略，不 panic |
| kitty 探测在慢终端超时导致启动延迟 | 50ms 窗口，超时即 fallback，不阻塞 |
| crossterm event::read 在探测窗口读到真实按键（用户在启动瞬间按键） | 探测窗口内读到的非响应事件被丢弃（可接受，50ms 内用户极少按键） |
