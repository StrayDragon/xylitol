# 10. 源码引用索引

| 文件路径 | 行号范围 | 章节 | 说明 |
|----------|----------|------|------|
| `packages/tui/src/tui.ts` | L39-L63 | 4 | Component 接口定义 |
| `packages/tui/src/tui.ts` | L74-L82 | 4 | Focusable 接口 |
| `packages/tui/src/tui.ts` | L84-L90 | 3 | CURSOR_MARKER 定义 |
| `packages/tui/src/tui.ts` | L97-L195 | 4 | Overlay 系统 (Options + Handle) |
| `packages/tui/src/tui.ts` | L200-L234 | 4 | Container 组合组件 |
| `packages/tui/src/tui.ts` | L239-L280 | 3 | TUI 类构造器与状态 |
| `packages/tui/src/tui.ts` | L253 | 3 | MIN_RENDER_INTERVAL_MS = 16 |
| `packages/tui/src/tui.ts` | L311-L323 | 4 | setFocus 焦点管理 |
| `packages/tui/src/tui.ts` | L329-L396 | 4 | showOverlay 实现 |
| `packages/tui/src/tui.ts` | L441-L449 | 3 | TUI.start() 启动流程 |
| `packages/tui/src/tui.ts` | L463-L471 | 3 | 终端能力查询 (cell size) |
| `packages/tui/src/tui.ts` | L495-L542 | 3 | requestRender + scheduleRender 防抖 |
| `packages/tui/src/tui.ts` | L544-L590 | 2 | handleInput 输入分发 |
| `packages/tui/src/tui.ts` | L933-L951 | 3 | extractCursorPosition |
| `packages/tui/src/tui.ts` | L953-L1280 | 3 | doRender 差分渲染核心 |
| `packages/tui/src/tui.ts` | L985-L994 | 3 | 同步输出包装 |
| `packages/tui/src/tui.ts` | L1029-L1032 | 3 | 宽度变化全量重绘 |
| `packages/tui/src/tui.ts` | L1038-L1042 | 3 | Termux 高度变化特殊处理 |
| `packages/tui/src/tui.ts` | L1054-L1067 | 3 | 行级 diff 算法 |
| `packages/tui/src/tui.ts` | L1145-L1209 | 3 | 增量写入 |
| `packages/tui/src/tui.ts` | L1180-L1207 | 8 | 行宽溢出保护 |
| `packages/tui/src/tui.ts` | L1287-L1318 | 3 | positionHardwareCursor IME 光标 |
| `packages/tui/src/terminal.ts` | L28-L70 | 2 | Terminal 接口定义 |
| `packages/tui/src/terminal.ts` | L75-L420 | 2 | ProcessTerminal 实现 |
| `packages/tui/src/terminal.ts` | L103-L137 | 2 | start() raw 模式 + 协议探测 |
| `packages/tui/src/terminal.ts` | L147-L194 | 2 | setupStdinBuffer |
| `packages/tui/src/terminal.ts` | L210-L220 | 2 | Kitty 协议探测与启用 |
| `packages/tui/src/terminal.ts` | L228-L256 | 2 | Windows VT 输入启用 |
| `packages/tui/src/terminal.ts` | L258-L294 | 8 | drainInput 退出前输入排空 |
| `packages/tui/src/terminal.ts` | L296-L341 | 2 | stop() 终端恢复 |
| `packages/tui/src/terminal.ts` | L398-L412 | 6 | setProgress 进度指示 |
| `packages/tui/src/stdin-buffer.ts` | L29-L78 | 2 | isCompleteSequence 状态机 |
| `packages/tui/src/stdin-buffer.ts` | L84-L126 | 2 | CSI 序列完整性检测 |
| `packages/tui/src/stdin-buffer.ts` | L192-L255 | 2 | extractCompleteSequences 拆分 |
| `packages/tui/src/stdin-buffer.ts` | L274-L434 | 2 | StdinBuffer 类 |
| `packages/tui/src/stdin-buffer.ts` | L315-L369 | 2 | Bracketed Paste 检测 |
| `packages/tui/src/stdin-buffer.ts` | L389-L398 | 2 | Kitty 重复消除 |
| `packages/tui/src/keys.ts` | L1-L80 | 2 | 按键解析 API |
| `packages/tui/src/keybindings.ts` | L7-L42 | 2 | Keybindings 声明式注册表 |
| `packages/tui/src/keybindings.ts` | L54-L80 | 2 | TUI_KEYBINDINGS 默认键位 |
| `packages/tui/src/components/editor.ts` | L1-L80 | 4 | Editor 组件 (imports + paste marker) |
| `packages/tui/src/components/editor.ts` | L101-L160 | 4 | wordWrapLine 智能换行 |
| `packages/tui/src/components/editor.ts` | L409-L532 | 4 | Editor.render() 含虚拟滚动 |
| `packages/tui/src/components/editor.ts` | L534-L650 | 4 | Editor.handleInput() 输入处理 |
| `packages/tui/src/components/editor.ts` | L1084-L1150 | 8 | handlePaste + tmux 重编码修复 |
| `packages/tui/src/components/editor.ts` | L1152-L1175 | 4 | addNewLine |
| `packages/tui/src/components/loader.ts` | L17-L92 | 6 | Loader spinner 动画 |
| `packages/coding-agent/.../interactive-mode.ts` | L568-L679 | 5 | init() UI 布局构建 |
| `packages/coding-agent/.../interactive-mode.ts` | L716-L789 | 5 | run() 主交互循环 |
| `packages/coding-agent/.../interactive-mode.ts` | L2376-L2440 | 2 | setupKeyHandlers 键绑定 |
| `packages/coding-agent/.../interactive-mode.ts` | L2645-L2839 | 5 | handleEvent 事件处理 |
| `packages/coding-agent/.../interactive-mode.ts` | L3025-L3115 | 4 | addMessageToChat |
| `packages/coding-agent/.../interactive-mode.ts` | L1964-L2018 | 7 | createExtensionUIContext |
| `packages/coding-agent/.../interactive-mode.ts` | L2024-L2062 | 7 | showExtensionSelector 对话框模式 |
| `packages/coding-agent/.../interactive-mode.ts` | L3229-L3237 | 6 | handleCtrlC 双击检测 |
| `packages/coding-agent/.../interactive-mode.ts` | L3285-L3302 | 8 | uncaughtCrash 终端恢复 |
| `packages/coding-agent/.../interactive-mode.ts` | L3358-L3393 | 6 | handleCtrlZ suspend/resume |
| `packages/coding-agent/.../interactive-mode.ts` | L3512-L3565 | 8 | openExternalEditor TUI 中断 |
| `packages/coding-agent/.../theme/theme.ts` | L29-L100 | 7 | 主题 JSON Schema (51 色值) |
| `packages/coding-agent/.../theme/theme.ts` | L167-L252 | 7 | 颜色降级 (truecolor → 256) |
| `packages/coding-agent/.../theme/theme.ts` | L289-L305 | 7 | 变量引用递归解析 |
| `packages/coding-agent/.../theme/theme.ts` | L714-L733 | 7 | detectTerminalBackground |
| `packages/coding-agent/.../theme/theme.ts` | L829-L900 | 7 | 主题热重载 (fs.watch) |
| `packages/coding-agent/.../theme/theme.ts` | L1163-L1227 | 7 | TUI 框架适配器接口 |
| `packages/coding-agent/src/core/output-guard.ts` | L9-L34 | 6 | takeOverStdout 重定向 |
| `packages/coding-agent/src/utils/clipboard.ts` | L35-L127 | 6 | 跨平台剪贴板降级链 |
| `packages/coding-agent/.../custom-editor.ts` | L7-L80 | 2 | CustomEditor 键绑定拦截 |
| `packages/coding-agent/.../assistant-message.ts` | L12-L147 | 5 | AssistantMessageComponent 流式渲染 |

> 所有文件路径相对于 monorepo 根目录 `pi-mono/`。`...` 是 `packages/coding-agent/src/modes/interactive/` 的缩写。
