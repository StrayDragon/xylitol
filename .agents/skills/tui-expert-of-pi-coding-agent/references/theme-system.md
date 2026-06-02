
### 主题定义

主题使用 JSON 文件定义，包含 51 个语义化色值，分为 8 组：

| 分组 | 色值数 | 示例 |
|------|--------|------|
| Core UI | 11 | accent, border, success, error, warning |
| Backgrounds | 6 | selectedBg, userMessageBg, toolSuccessBg |
| Markdown | 10 | mdHeading, mdLink, mdCodeBlock |
| Tool Diffs | 3 | toolDiffAdded, toolDiffRemoved |
| Syntax | 9 | syntaxKeyword, syntaxFunction, syntaxString |
| Thinking Borders | 6 | thinkingOff..thinkingXhigh |
| Bash Mode | 1 | bashMode |
| Export | 3 | pageBg, cardBg, infoBg |

`theme.ts#L29-L100`

### 颜色降级

支持 truecolor → 256color 自动降级：

- Truecolor: `\x1b[38;2;R;G;Bm`
- 256-color: 通过加权欧几里得距离找最近色彩立方体或灰度值 `theme.ts#L221-L252`
- 灰度优先: 当色彩饱和度极低 (spread < 10) 时优先使用灰度 `theme.ts#L247-L249`

### 主题热重载

通过 `fs.watch` 监听主题文件变化，100ms 防抖后重新加载主题并调用 `invalidate()` 刷新所有组件缓存。仅自定义主题（非 dark/light 内置主题）启用热重载。`theme.ts#L829-L900`

### 变量引用

主题支持 `vars` 字段定义变量，颜色值可引用变量名（如 `primary`），解析时递归展开为实际颜色值，检测循环引用。`theme.ts#L289-L305`

### 终端背景检测

`detectTerminalBackground()` 自动检测终端背景色以选择 dark/light 主题：

1. 检查 `COLORFGBG` 环境变量的最后一段（背景色 index），计算亮度 `theme.ts#L714-L733`
2. 运行时可通过 OSC 11 (`\x1b]11;?`) 查询终端背景 RGB 值 `theme.ts#L682-L712`
3. 无法检测时默认 dark 主题

### TUI 框架适配器

Theme 类为各 TUI 框架组件提供适配接口：

- `getEditorTheme()`: Editor 边框/选择列表颜色 `theme.ts#L1212-L1217`
- `getMarkdownTheme()`: Markdown 渲染器的色彩回调集 `theme.ts#L1163-L1200`
- `getSelectListTheme()`: 选择列表的前缀/高亮色 `theme.ts#L1202-L1210`
- `getSettingsListTheme()`: 设置编辑器的标签/值/光标色 `theme.ts#L1219-L1227`

### Extension UI Context 扩展点

`InteractiveMode` 为扩展提供了完整的 UI 操控接口 `ExtensionUIContext`，覆盖对话框、组件注入、主题控制等：

```typescript
// interactive-mode.ts#L1964-L2018 (createExtensionUIContext)
interface ExtensionUIContext {
    select(title, options, opts?): Promise<string | undefined>;  // 选择对话框
    confirm(title, message, opts?): Promise<boolean>;            // 确认对话框
    input(title, placeholder?, opts?): Promise<string | undefined>; // 输入对话框
    editor(title, prefill?): Promise<string | undefined>;        // 多行编辑器
    notify(message, type?): void;                                // 通知消息
    custom<T>(factory, options?): Promise<T>;                    // 自定义组件
    onTerminalInput(handler): () => void;                        // 拦截原始输入
    setWidget(key, component, options?): void;                   // 注入 widget
    setFooter(factory): void;                                    // 替换底栏
    setHeader(factory): void;                                    // 替换顶栏
    setEditorComponent(factory): void;                           // 替换编辑器
    setTheme(themeOrName): { success: boolean; error?: string }; // 切换主题
    pasteToEditor(text): void;                                   // 模拟粘贴到编辑器
    setWorkingMessage(message): void;                            // 自定义工作提示
    setStatus(key, text): void;                                  // 底栏状态文本
}
```

对话框通过 Promise + editorContainer 替换实现：显示时用选择器替换编辑器并转移焦点，关闭时恢复编辑器。支持 `AbortSignal` 和超时取消。`interactive-mode.ts#L2024-L2062`

自定义组件支持两种模式：`overlay` 模式渲染在内容之上（用于游戏、全屏预览等），普通模式替换编辑器区域。

---
