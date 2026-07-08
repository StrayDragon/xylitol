# Design — c410-complete-terminal-protocol

> 本变更有一个核心架构权衡（pi 事件模型 vs xy crossterm 模型）+ 探测窗口设计决策。

## 决策 1：路线 A（crossterm + 探测窗口）而非路线 B（自管字节流）

### 背景

pi 的 `ProcessTerminal` **拥有原始 stdin 字节流**：它注册 `process.stdin.on('data')`，经 `StdinBuffer` 拆分，在转发给组件前拦截 Kitty 协议响应（`keyboardProtocolNegotiationBuffer` 状态机）。

xy 用 crossterm 的 `event::read()`，crossterm **已经把字节流解析成 `KeyEvent`/`Event::Paste`/`Event::Resize`**。xy 从不接触原始字节。

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. crossterm + 探测窗口** | start 时发 Kitty 查询，poll 窗口内用 event::read 抓响应 | ✅ 采用（用户定）|
| B. 自管原始字节流 | 新建 RawStdinReader，移植 pi 的 StdinBuffer 拦截 + 状态机 | ❌ 拒绝（改动大，放弃 crossterm 输入层）|
| C. 输出侧补齐，协商推迟 | 只做 modifyOtherKeys/OSC，Kitty 探测不做 | ❌ 拒绝（keys.rs 的 kitty 分支永远不激活）|

### 理由

- **B 过度**：crossterm 已经做了输入解析（KeyEvent、bracketed paste、resize），自管字节流要重做这些，且放弃 crossterm 的跨平台能力（Windows）。pi 自管是因为 Node.js 的 stdin 是裸字节流。
- **C 不达标**：keys.rs 的 `is_kitty_protocol_active()` 分支已经存在但永不激活，等于死代码。terminal.rs 不探测，后续 editor/autocomplete 移植假设的 kitty 能力就是假的。
- **A 务实**：保留 crossterm 全部能力，只在 start 瞬间用 event::poll/read 做一次探测。Kitty 激活尽力而为（终端在 50ms 内响应即激活，否则 fallback）。

### 后果

- Kitty 激活不是 100% 可靠（慢终端可能超时）。但 fallback 到 modifyOtherKeys 仍有增强（比现状的「什么都不做」强）。
- 探测窗口内（50ms）用户按键会被丢弃——可接受（启动瞬间极少按键）。

## 决策 2：探测窗口用 event::read 抓 CSI 响应

### 问题

crossterm 把 Kitty 协议响应 `CSI ?7u` 当成什么 Event？可能：
- `Event::Key(KeyEvent)`（crossterm 尝试解析成按键）
- 未知 Event（crossterm 不识别则可能返回错误或丢弃）

### 设计

探测窗口内：
1. `event::poll(50ms)` → 超时则 fallback
2. `event::read()` → 得到 Event
3. 尝试从 Event 提取原始字节——但 crossterm 的 Event 不暴露原始字节！

**这是路线 A 的关键限制**：crossterm 消费了原始字节，我们拿不到 `CSI ?7u` 的字面文本。

### 解法：基于可用信号判断

crossterm 0.29 对 Kitty 协议响应的处理：CSI ?Nu 不是有效按键，crossterm 会把它解析成某种 Key（或丢弃）。我们无法可靠拿到 flags。

**务实方案**：探测改为「发查询 + 看终端是否在窗口内回显了任何数据」。具体：
- 发 `CSI >7u CSI ?u CSI c`
- poll 窗口内：如果 `event::poll` 返回 true（有数据到达，无论 crossterm 解析成什么），认为终端**响应了**（支持某种协议）→ 但无法区分 kitty vs DA。
- 进一步：crossterm 0.29 支持 `KeyEvent::kind`（press/repeat/release），如果探测窗口后收到的按键带 `Kind::Release`，说明 kitty 协议生效了（只有 kitty 才报 release）。

**但这个判断不可靠**。所以最终采用更简单的策略：

### 最终策略：发查询序列 + 信任 crossterm 的 kitty 支持

crossterm 0.29 自身会与终端协商 kitty 协议（`PushKeyboardEnhancementFlags`）。xy 发查询序列是**对齐 pi 的行为**（让终端进入增强模式），但**激活判断交给 crossterm 的能力**：xy 不自己解析 flags，而是在 start 后发 `PushKeyboardEnhancementFlags(ALL)`，crossterm 自己决定是否生效。

如果 crossterm 的 `push_enhancement_flags` 成功（终端支持），xy 就 `set_kitty_protocol_active(true)`。

这比「自己解析 CSI 响应」可靠得多（crossterm 做了实际工作），且符合路线 A 的「保留 crossterm 输入层」精神。

### 后果

- `parse_kitty_flags` 仍作为**纯函数**实现 + 单测（对齐 pi 的 `parseKeyboardProtocolNegotiationSequence`，供未来路线 B 或手动诊断用），但**运行时探测走 crossterm 的 push_enhancement_flags**。
- spec tp01 的「探测窗口读响应」调整为「发查询 + 调 crossterm push_enhancement_flags 据返回值激活」。

## 决策 3：OSC 进度不做 keepalive

pi 用 1s interval 重发 `OSC 9;4;3`（某些终端会超时清除进度）。xy 不做——TUI 是长驻进程，退出时清一次即可。如果未来发现某终端需要 keepalive，再加。

## 风险

| 风险 | 缓解 |
|---|---|
| crossterm push_enhancement_flags 在某些终端报错 | 捕获错误，fallback 到 modifyOtherKeys |
| 探测失败导致 kitty 永不激活 | fallback 链：kitty → modifyOtherKeys → 啥都不做（比现状好）|
| 测试无法验证真实 kitty 激活 | 第 5a 层 E2E 验证「查询序列被发出」+ 单测验证 parse_kitty_flags 纯函数 |

## 非目标

- 不自管 stdin 字节流（路线 B）
- 不移植 StdinBuffer 的响应拦截状态机
- 不做 Apple Terminal 归一化 / Windows VT input（crossterm 已覆盖）
- 不做 write log（用 tracing）
