# 2. 事件循环与输入处理

### 事件循环实现

TUI 没有显式事件循环。它依赖 Node.js 事件驱动架构：

1. **stdin data 事件** → `StdinBuffer.process(data)` → 拆分为完整序列 → `handleInput(data)` `terminal.ts#L103-L137`
2. **resize 事件** → `stdout.on("resize", handler)` → `requestRender()` `terminal.ts#L119`
3. **定时器** → `Loader` 组件的 `setInterval` 驱动动画，每次 tick 调用 `requestRender()` `loader.ts#L77-L81`

### StdinBuffer: 输入拆分与粘贴检测

核心问题是 stdin data 事件可能包含不完整的转义序列。`StdinBuffer` 通过状态机+超时机制解决：

- **序列完整性检测**: 逐字符判断 CSI/OSC/DCS/APC/SS3 序列是否完整 `stdin-buffer.ts#L29-L78`
- **Bracketed Paste**: 检测 `\x1b[200~` ... `\x1b[201~` 包裹的粘贴内容，作为单一 `paste` 事件发出 `stdin-buffer.ts#L315-L369`
- **超时 flush**: 10ms 内序列未闭合则强制发出，避免 ESC 键等待过长 `stdin-buffer.ts#L378-L386`
- **Kitty 重复消除**: Kitty 协议会同时发送 printable codepoint 和传统序列，buffer 去重以避免双击 `stdin-buffer.ts#L389-L398`

### Kitty 键盘协议与降级

`ProcessTerminal` 在启动时探测 Kitty 协议支持：

1. 发送 `\x1b[?u` 查询当前 flags `terminal.ts#L213`
2. 若终端响应 `\x1b[?<flags>u`，启用 Kitty 协议（flags: disambiguate + report events + alternate keys） `terminal.ts#L156-L168`
3. 150ms 无响应则降级到 xterm `modifyOtherKeys` mode 2 (`\x1b[>4;2m`) `terminal.ts#L214-L219`
4. 退出时正确还原所有模式 `terminal.ts#L296-L341`

### 键映射与命令模式

`KeybindingsManager` 提供声明式键绑定：

- **全局注册表**: 通过 TypeScript 声明合并（`interface Keybindings`）定义所有可用动作 `keybindings.ts#L7-L42`
- **默认映射**: `TUI_KEYBINDINGS` 常量提供默认键位 `keybindings.ts#L54-L80`
- **用户覆盖**: 支持用户自定义键位，冲突检测 `keybindings.ts#L194-L212`
- **多键绑定**: 一个动作可绑定多个键（如 `["left", "ctrl+b"]`） `keybindings.ts#L57-L59`

### 一次按键到状态变更

```mermaid
sequenceDiagram
    participant T as Terminal
    participant SB as StdinBuffer
    participant TUI as TUI.handleInput()
    participant IL as InputListeners
    participant ED as Editor.handleInput()
    participant KB as KeybindingsManager
    participant App as InteractiveMode

    T->>SB: stdin data: "\x1b[1;5C" (Ctrl+Right)
    SB->>TUI: emit "data", "\x1b[1;5C"
    TUI->>IL: 遍历 listeners (可拦截/转换)
    IL-->>TUI: { consume: false }
    TUI->>ED: focusedComponent.handleInput(data)
    ED->>KB: matches(data, "tui.editor.cursorWordRight")
    KB-->>ED: true
    ED->>ED: moveWordForwards()
    ED->>App: onChange(newText)
    ED->>TUI: requestRender() [隐式]
    TUI->>TUI: scheduleRender() [16ms 防抖]
    TUI->>TUI: doRender() [差分渲染]
```

---
