# pi TUI 调研报告

## 0. 元信息

- **技术栈**: Ink (React for CLI) 的自制替代: `packages/tui/` 用纯 TS + 差分渲染实现的组件系统,无 React 依赖
- **组件入口**: `packages/coding-agent/src/modes/interactive/components/` (约 30 个组件)
- **底层 TUI 包**: `packages/tui/src/` — 组件基类 (`Component`/`Focusable`),容器的 `Container`/`Box`/`Text`/`Spacer`/`Markdown`,及交互组件 `Input`/`Editor`/`SelectList`/`SettingsList`/`Loader`
- **渲染方式**: 差分渲染。`render(width) => string[]` 每帧返回字符串数组,TUI 对比上一帧只写差异行到终端。不依赖任何 virtual-DOM/libs。
- **报告日期**: 2026-07-02

---

## 1. 组件全景清单

| 组件名 | 类别 | 文件 | 职责 | 对 xylitol 可借鉴度 |
|--------|------|------|------|---------------------|
| `AssistantMessageComponent` | 消息 | `assistant-message.ts` | 渲染 assistant 回复(含 thinking 折叠、stop reason 错误) | 高 — 流式 text+thinking 分段是 LLM TUI 核心 |
| `UserMessageComponent` | 消息 | `user-message.ts` | 用户消息回显,带背景色、Markdown | 高 |
| `BashExecutionComponent` | chrome/消息 | `bash-execution.ts` | bash 命令带边框、spinner、流式输出、折叠/展开 | 高 — 工具执行 chrome 核心 |
| `ToolExecutionComponent` | chrome/消息 | `tool-execution.ts` | 通用工具执行渲染(含 call/result 双渲染器、图像内联) | 高 — 可扩展渲染很重要 |
| `renderDiff` (非组件,函数) | 消息 | `diff.ts` | 行级 + 词级 diff 渲染,彩色 | 高 — diff 是 coding agent 核心交互 |
| `FooterComponent` | chrome | `footer.ts` | 状态栏: pwd, token 统计, 上下文使用率, git branch, 模型名, 扩展状态 | 高 |
| `DynamicBorder` | chrome | `dynamic-border.ts` | 终端宽度的自适应边框 `─` 线 | 中 — ratatui 自带 `Block` |
| `BorderedLoader` | chrome | `bordered-loader.ts` | 带边框的 spinner + 可取消 | 中 — ratatui `Paragraph`+ 自制 spinner |
| `CustomEditor` | 输入 | `custom-editor.ts` | 继承 `Editor`,处理 app-level 键绑定 | 中 — ratatui 需自建多行输入 |
| `ExtensionEditorComponent` | 输入 | `extension-editor.ts` | 扩展注入的自定义编辑器壳 | 低 — 扩展点概念,非基础 |
| `ExtensionInputComponent` | 输入 | `extension-input.ts` | 扩展注入的单行输入 | 低 |
| `ModelSelectorComponent` | 选择器 | `model-selector.ts` | 模型选择:搜索 + 列表 + scope 切换 | 高 — 搜索型选择器范式 |
| `SessionSelectorComponent` | 选择器 | `session-selector.ts` | 会话选择:树形/平铺、搜索、排序、scope、删除确认、重命名 | 高 — 最复杂的选择器,范式丰富 |
| `SettingsSelectorComponent` | 选择器 | `settings-selector.ts` | 设置面板:二值切换、子菜单、theme 子菜单支持预览 | 高 — 值+子菜单范式 |
| `ThinkingSelectorComponent` | 选择器 | `thinking-selector.ts` | 思维深度选择 | 中 — SelectList 封装 |
| `ThemeSelectorComponent` | 选择器 | `theme-selector.ts` | 主题选择 | 中 |
| `TreeSelectorComponent` | 选择器 | `tree-selector.ts` | 会话树导航:折叠/展开、ASCII 树连接线、过滤、水平滚动、搜索、label 编辑 | 高 — 树形导航复杂范式 |
| `ScopedModelsSelectorComponent` | 选择器 | `scoped-models-selector.ts` | 作用域模型选择(工具绑定的模型) | 低 — ModelSelector 变体 |
| `UserMessageSelectorComponent` | 选择器 | `user-message-selector.ts` | 用户消息历史选择(编辑/重发) | 低 |
| `OAuthSelectorComponent` | 选择器 | `oauth-selector.ts` | OAuth provider 选择 | 低 |
| `ShowImagesSelectorComponent` | 选择器 | `show-images-selector.ts` | 图片显示开关 | 低 |
| `TrustSelectorComponent` | 选择器 | `trust-selector.ts` | 项目信任决策 | 低 |
| `ExtensionSelectorComponent` | 选择器 | `extension-selector.ts` | 扩展列表选择 | 低 |
| `LoginDialogComponent` | 对话框 | `login-dialog.ts` | OAuth 登录流程:弹窗、验证码显示、输入 | 中 — 多步骤对话框 |
| `FirstTimeSetupComponent` | 对话框 | `first-time-setup.ts` | 首次运行向导:theme + 遥测设置,多步 | 中 — 多步骤对话框 |
| `CustomMessageComponent` | 消息 | `custom-message.ts` | 扩展自定义消息渲染 | 中 — 扩展消息渲染 |
| `SkillInvocationMessageComponent` | 消息 | `skill-invocation-message.ts` | 技能调用消息 | 低 |
| `CompactionSummaryMessageComponent` | 消息 | `compaction-summary-message.ts` | 上下文压缩摘要 | 中 — 展示"发生了什么" |
| `BranchSummaryMessageComponent` | 消息 | `branch-summary-message.ts` | 分支切换摘要 | 低 |
| `ArminComponent` | media/easter egg | `armin.ts` | 动画 XBM 字符画(彩蛋) | 低 — 纯消遣 |
| `DaxnutsComponent` | media | `daxnuts.ts` | 又一个字符画 | 低 |
| `keyHint` / `keyText` / `rawKeyHint` | 工具函数 | `keybinding-hints.ts` | 格式化键绑定提示文本 | 中 — 键提示格式工具 |
| `visual-truncate` (函数) | 工具函数 | `visual-truncate.ts` | 按视觉宽度截断 | 中 — 宽度感知截断 |
| `keybinding-hints.ts` | 工具函数 | `keybinding-hints.ts` | 键绑定显示格式化 | 中 |

**分类统计:**
- **消息渲染** (8): AssistantMessage, UserMessage, CustomMessage, SkillInvocation, CompactionSummary, BranchSummary, BashExecution, ToolExecution
- **输入** (3): CustomEditor, ExtensionEditor, ExtensionInput
- **选择器** (12): ModelSelector, SessionSelector, SettingsSelector, ThinkingSelector, ThemeSelector, TreeSelector, ScopedModelsSelector, UserMessageSelector, OAuthSelector, ShowImagesSelector, TrustSelector, ExtensionSelector
- **对话框** (2): LoginDialog, FirstTimeSetup
- **chrome** (3): Footer, DynamicBorder, BorderedLoader
- **media/easter egg** (2): Armin, Daxnuts
- **工具函数** (3): keyHint, visual-truncate, keybinding-hints

---

## 2. 交互形态

### 流式 assistant 消息

**组件**: `AssistantMessageComponent`

**结构**:
1. 消息内容按顺序排列: `thinking` block → `text` block → `toolCall` block
2. 每个 text block 用 `Markdown` 组件渲染(带语法高亮)
3. thinking block 有两种状态:
   - **展开**: 用 `Markdown` + `italic` + `thinkingText` 颜色渲染
   - **折叠**: 显示静态文字 "Thinking..."
4. 消息末尾显示 `stopReason` 错误:
   - `"length"` → 红色错误(超出 token 限制)
   - `"aborted"` → 红色 "Operation aborted"
   - `"error"` → 红色错误详情

**关键细节**:
- OSC 133 标记包裹边界(`\x1b]133;A\x07` 开始, `\x1b]133;B\x07` + `\x1b]133;C\x07` 结束) — 终端语义标记
- 有 toolCall 时不加 OSC 133 包裹(工具执行由子组件独立渲染)
- 外层 `Container` 组合: `Spacer` → `Markdown` × N → `Spacer`(若需) → 错误文字

**对 ratatui 的启示**: 这本质是一个 `Vec<Widget>` 的组合。ratatui 可以用 `Layout::vertical` + 多个 `Paragraph` 实现同样的布局。thinking 折叠/展开状态在 app state 中维护。

### 工具执行展示

**组件**: `ToolExecutionComponent` + `BashExecutionComponent`

**ToolExecutionComponent (通用工具渲染器)**:

架构核心是 **双阶段渲染器**:
- `renderCall(args, theme, context)` — 渲染工具调用阶段(参数展示)
- `renderResult(result, options, theme, context)` — 渲染结果阶段(输出展示)

渲染器由工具定义(`ToolDefinition`)提供。内置工具(bash/read/write/edit)有自己的渲染器,扩展可以覆盖。

状态驱动显示:
- **调用中**: `toolPendingBg` 背景色 + 调用渲染器
- **成功**: `toolSuccessBg` 背景色 + 结果渲染器
- **错误**: `toolErrorBg` 背景色 + 错误信息

**渲染壳(renderShell)**: 两种布局模式:
- `"default"`: 内容在 `Box` 内(带一致背景色)
- `"self"`: 工具渲染器自行管理整个布局(如 bash 的边框)

**进展更新**: `updateArgs()` → `setArgsComplete()` → `markExecutionStarted()` → `updateResult(result, isPartial)`

支持:
- 内联图片(Kitty 协议)
- 展开/折叠
- 跨渲染器状态共享(`rendererState`)

**BashExecutionComponent (bash 专用)**:

边框包裹布局:
```
┌────────────────  ← DynamicBorder
│ $ command        ← 命令行(粗体)
│ output line 1    ← 输出(流式追加, ANSI 剥离)
│ output line 2
│ ⠋ Running...     ← Loader spinner
│ esc to cancel    ← 取消提示
└────────────────  ← DynamicBorder
```

完工后:
- 展开/折叠输出(预览默认 20 行)
- 状态行: `(exit code)` / `(cancelled)`
- 截断提示: `... N more lines (to expand)` / `Output truncated. Full output: /tmp/...`
- `TruncationResult` 表示 LLM 上下文截断的信息

**对 ratatui 的启示**:
- `ToolExecutionComponent` 的 `renderCall`/`renderResult` 双阶段模式值得在 xylitol 中用 trait 实现
- bash 的边框布局可以直接映射到 ratatui 的 `Block::bordered()` + `Paragraph`
- 流式输出用 `Paragraph` + `Scrollbar` 实现,或自己维护行 buffer

### diff 渲染

**组件/函数**: `renderDiff()` (非 Component,接受 diff 文本返回 ANSI 字符串)

**算法**:
1. 解析统一 diff 行格式: `^([+-\s])(\s*\d*)\s(.*)$`
2. 颜色映射:
   - 上下文行: `toolDiffContext` (dim)
   - 删除行: `toolDiffRemoved` (红)
   - 添加行: `toolDiffAdded` (绿)
3. **行内差额对比**: 当恰好一对 `-` / `+` 连续出现,使用 `diff.diffWords()` 做词级比较
   - 变化部分用 `inverse` 高亮
4. 多行增减不做词级对比,只做行级着色

**对 ratatui 的启示**: 这个函数可以原样移植为 Rust 函数,输出一个 `Vec<Span>` 或 `Vec<Line>`。词级 diff 使用 `similar` crate。

### 用户消息回显

**组件**: `UserMessageComponent`

- 用 `Box` 包裹内容,背景色为 `userMessageBg`
- `Markdown` 内容使用 `userMessageText` 前景色
- 也包裹 OSC 133 标记
- 布局: `Spacer(1) → Box(1,1, bg=userMessageBg)`

**对 ratatui 的启示**: 直接映射为 `Paragraph` + `Block` 加背景色。无特殊。

---

## 3. 对话框/选择器范式

### 共性范式

所有选择器都实现 `Component` 接口,有的同时实现 `Focusable`(用于 IME 光标定位)。在 `Container` 内组合:

```
[DynamicBorder]  ─── 上边框
[标题文字]
[输入框]         ─── 可选搜索
[选择列表]       ─── 选项列表
[滚动指示器]
[帮助文字]       ─── 键绑定提示
[DynamicBorder]  ─── 下边框
```

**调用方式**: 通过 `ctx.ui.custom(component, { overlay: true })` 以叠加层弹出。返回 `Promise<T>`,用户关闭时 resolve。

### ModelSelectorComponent

弹出流程:
1. 加载模型列表(异步,有加载状态)
2. 搜索框已聚焦,按文字实时过滤
3. 导航: `↑`/`↓` 选择,循环到底
4. `Enter` 选中 → 保存为默认模型 → 关闭 → `onSelect` 回调
5. `Esc`/`Ctrl+C` → 取消 → `onCancel`
6. `Tab` 切换 scope (`all` / `scoped`)

展示: `→ model_id [provider] ✓` (选中行加 `accent` 颜色,当前模型加 `✓`)

### SessionSelectorComponent

最复杂的选择器,功能丰富:

**scope 切换**: `current`(当前目录) ↔ `all` 加载所有会话

**排序模式**: `threaded`(树形,按 parentSessionPath 构建) → `recent`(按修改时间) → `relevance`(按搜索相关度)

**名称过滤**: `all` ↔ `named`(仅有名会话)

**搜索**: 输入实时过滤 + 支持 `re:<pattern>` 正则 + `"phrase"` 精确匹配

**树形显示**: `├─`/`└─` ASCII 连接线,`│` 竖线表示嵌套层级

**动作**:
- `Ctrl+D` 删除(二次确认: 按 `Enter` 确认/`Esc` 取消)
- `Ctrl+R` 重命名(切换到 rename 子模式,内联输入框)
- `Ctrl+P` 切换路径显示
- 所有操作有 Toast 式状态反馈(`setStatusMessage`)

### SettingsSelectorComponent

多层级设置面板:
- 二值切换: `true`/`false` 通过枚举值循环
- 多值选择: 预定义选项列表
- **子菜单**: 选择某个设置项 → Enter → 弹出更精细的选择器(如 theme 子菜单,再套子菜单选择 light/dark theme)
- 搜索: `enableSearch: true` 启用模糊搜索
- 实时生效,不等待关闭

### TreeSelectorComponent

会话树导航,最繁复:

**树结构**: 从 `SessionTreeNode[]` 构建 (根节点 + children 递归)

**过滤模式** (5 种): `default` / `no-tools` / `user-only` / `labeled-only` / `all`,通过键循环切换

**水平滚动**: 当选中行偏移到右侧时自动水平滚动,保持 anchor 可见

**折叠/展开**: `app.tree.foldOrUp` / `app.tree.unfoldOrDown` 键折叠子树(类似 VSCode 大纲)

**标签编辑**: 进入编辑模式 → 内联 `Input` 组件 → Enter 保存 / Esc 取消

**搜索**: 直接输入文字即过滤(无搜索框,按字符累积),Backspace 清除

**键绑定** (6 组):
- `↑/↓`: 移动
- `←/→`: 翻页
- `app.tree.foldOrDown`: 分支导航
- `app.tree.editLabel`: 编辑标签
- `␣`: 切换标签时间戳显示
- `f`: 循环过滤模式

### 共同的设计模式

1. **选择+搜索一体**: 大多选择器把搜索输入和选择列表放在同一个组件内,键盘输入自动过滤
2. **异步加载**: Model / Session 数据异步加载,加载中显示 spinner 或 progress
3. **二层确认**: 危险操作(删除)要求再次确认
4. **子模式**: 内联子模式(rename / submenu)通过切换 container 内容实现
5. **统一的 overlay 生命周期**: `ctx.ui.custom()` → Promise-based,关闭后 dispose

---

## 4. 测试模式

### pi 怎么测 Ink 组件

**文件**: `packages/coding-agent/test/tool-execution-component.test.ts`

**测试框架**: Vitest

**核心模式**:
1. 创建真实的组件实例(不是 mock)
2. 传入 fake 但符合类型的参数
3. 调用 `component.render(width)`
4. `stripAnsi()` 去掉 ANSI 后断言文本内容

### fake 数据怎么造

```typescript
// Fake TUI 对象(只需要实现接口的极小部分)
function createFakeTui(): TUI {
  return {
    requestRender: () => {},
  } as unknown as TUI;
}

// Fake ToolDefinition
function createBaseToolDefinition(name = "custom_tool"): ToolDefinition {
  return {
    name,
    label: name,
    description: "custom tool",
    parameters: Type.Any(),
    execute: async () => ({
      content: [{ type: "text", text: "ok" }],
      details: {},
    }),
  };
}
```

### tool-execution-component.test.ts 模式摘要

1. **渲染器堆叠**: 验证 `renderCall` + `renderResult` 同时在渲染输出中出现
2. **self-render 空结果**: 验证空输出时组件返回 `[]` 不占空间
3. **内置工具回退**: 自定义 override 没有自定义渲染器时,回退到内置工具渲染器
4. **内置工具文件路径兼容**: 验证旧格式 `file_path` 正常渲染
5. **bash 流式**: bash 工具在 output 到达前先发一个空的 partial update
6. **截断不重复**: 输出截断提示只出现一次,且格式正确
7. **渲染器状态共享**: `renderCall` 中设置 `context.state.token`, `renderResult` 中读取
8. **展开/折叠**: 验证 collapsed 时隐藏内容,expanded 时显示
9. **特殊路径紧凑显示**: SKILL.md / AGENTS.md / Pi 文档路径紧凑渲染,expanded 才显示完整内容

**测试断言风格**:
```typescript
expect(stripAnsi(component.render(120).join("\n"))).toContain("custom call");
expect(stripAnsi(component.render(120).join("\n"))).not.toContain("hidden content");
expect(rendered.match(/\bread\b/g)?.length ?? 0).toBe(1); // 唯一性验证
```

---

## 5. 可视化示意(TUI 字符界面)

### ① 流式对话中(带工具执行)

```
┌─────────────────────────────────────────────────────┐
│                                                      │
│  user message content here                           │
│ ─────────────────────────────────────────────────     │  ← UserMessageComponent
│                                                      │
│  Let me check the codebase for this...               │
│                                                      │
│  _The model is reasoning about the approach..._      │  ← thinking block (italic)
│                                                      │
│  I'll look at the relevant file first.                │
│                                                      │
│ ┌─────────────────────────────────────────────────┐  │
│ │ $ rg -n "important" src/                        │  │  ← BashExecutionComponent
│ │ src/main.rs:42:fn important_function() {        │  │     running state
│ │ src/lib.rs:15: // important config               │  │
│ │ ⠋ Running... (esc to cancel)                     │  │     + Loader spinner
│ └─────────────────────────────────────────────────┘  │
│                                                      │
│ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄       │  ← ThinkingLoader
│                                                      │
│ ~/project (main)                                     │  ← FooterComponent
│ ↑1.2k ↓0.5k R0.3k CH25.0% $0.012 45%/128k auto     │      pwd + token stats + context %
│                                                      │      + model name on right
└─────────────────────────────────────────────────────┘
```

**组件**: UserMessageComponent, AssistantMessageComponent (含 thinking), BashExecutionComponent (running), FooterComponent

**用户处于**: 等待 bash 命令完成,模型正在思考下一步

### ② 选择器弹出(SessionSelector)

```
┌─────────────────────────────────────────────────────┐
│  pi session (streaming content above...)             │
│                                                      │
│ ╭───────────────────────────────────────────────╮    │  ← overlay
│ │ ─────────────────────────────────────────────  │    │  ← DynamicBorder
│ │ Resume Session (Current Folder)                │    │     SessionSelectorHeader
│ │ ○ Current Folder | ◉ All     Name: All Sort:Thd│    │
│ │ ─────────────────────────────────────────────  │    │
│ │ search: [my_feature                           ]│    │  ← Input (search)
│ │                                                 │    │
│ │  › my_feature_branch  3 2h                      │    │  ← SessionList
│ │    refactor_utils     1 1d                      │    │     选中行加→和 selectedBg
│ │    fix/login          5 3d                      │    │
│ │    ├─ fix/login-v2    2 4d                      │    │     树形连接线(threaded mode)
│ │    └─ fix/login-oauth 1 5d                      │    │
│ │    wip/api-docs       2 1w                      │    │
│ │      (8/12)                                     │    │     滚动指示器
│ │                                                 │    │
│ │ tab scope · re:<pattern> regex · "phrase" exact │    │
│ │ f sort · n named · d delete · p path (off)      │    │
│ │ ─────────────────────────────────────────────  │    │
│ ╰───────────────────────────────────────────────╯    │
│                                                      │
│ _                                                    │  ← editing area still visible
└─────────────────────────────────────────────────────┘
```

**组件**: SessionSelectorComponent (含 SessionList, SessionSelectorHeader, Input, DynamicBorder x2)

**用户处于**: 打开了会话选择器覆盖层,正在搜索/选择要恢复的会话。主界面内容在覆盖层后保持可见。

### ③ Diff 展示

```
│ ┌─────────────────────────────────────────────────┐  │
│ │ edit                                            │  │  ← ToolExecutionComponent
│ │ README.md                                       │  │     (edit 工具)
│ │                                                 │  │
│ │ ── src/main.rs ────────────────────────────────  │  │
│ │  -12 fn handle_old(data: &str) -> Result<()>     │  │  ← 删除行(红)
│ │  +12 fn handle_new(data: &[u8]) -> Result<()>   │  │  ← 添加行(绿)
│ │  -13     let x = parse_old(data);                │  │     词汇变化: inverse 高亮
│ │  +13     let x = parse_new(data);               │  │
│ │   14     process(x)                              │  │  ← 上下文行(dim)
│ │   15 }                                           │  │
│ │                                                 │  │
│ │  (esc to collapse)                               │  │  ← 展开/折叠提示
│ └─────────────────────────────────────────────────┘  │
```

**组件**: ToolExecutionComponent (使用 edit 工具的内置 renderCall/renderResult), renderDiff 函数

**用户处于**: 查看 edit 操作的结果,diff 展开显示完成。颜色标记: 红=删除,绿=添加,inverse=词级变化,灰色=上下文行。

---

## 6. 对 xylitol 的启示(ratatui 翻译)

### 6.1 核心架构借鉴

| pi 范式 | ratatui 等效 | 说明 |
|--------|-------------|------|
| `Component.render(width) => string[]` | `Widget.render(area, buf)` | ratatui 原生 widget 模型,但 pi 的差分布局技术值得借鉴 |
| `Container`(垂直堆叠) | `Layout::vertical` + `Constraint` | 等效,ratatui 更灵活 |
| `Box`(带背景和 padding 的容器) | `Block::bordered()` + `widgets::Paragraph` | 等效 |
| `Composite`(树状容器嵌套) | 层层 Layout | ratatui 布局更显式,但更容易控制 |

### 6.2 高价值复刻交互

#### (1) 流式消息渲染框架
- **pi**: `AssistantMessageComponent` 处理 thinking/text/toolCall 顺序
- **xylitol**: 需要一个 `MessageRenderer` trait,根据 `MessagePart` 枚举渲染不同 widget:
  ```rust
  enum MessagePart {
    Text(String),
    Thinking(String, ThinkingState), // ThinkingState: Expanded | Collapsed
    ToolCall(ToolCallMeta),
  }
  ```
- **ratatui 积木**: `Paragraph` + `Layout::vertical`, thinking 折叠用 app state 控制

#### (2) 工具执行双阶段渲染
- **pi**: `ToolDefinition.renderCall` + `renderResult`,可扩展
- **xylitol**: 定义 trait:
  ```rust
  trait ToolRenderer {
      fn render_call(&self, args: &Args, area: Rect, buf: &mut Buffer);
      fn render_result(&self, result: &Result, options: &RenderOptions, area: Rect, buf: &mut Buffer);
  }
  ```
- 每个工具注册自己的渲染器,框架提供默认 fallback
- **ratatui 积木**: 自建 trait,ratatui-widgets 无等价物

#### (3) 搜索型选择器
- **pi**: `ModelSelectorComponent` / `SessionSelectorComponent` — 搜索框 + 列表 + 异步数据
- **xylitol**: 实现 `SelectableList<T>` 组件:
  - 自带搜索输入
  - 泛型项: `T: Display + Searchable`
  - 导航: arrow keys, page up/down, home/end
  - 异步数据: 通过回调或 channel
- **ratatui 积木**: `List` widget,搜索条用 `Paragraph` 自制,`Input` 需自建

#### (4) 树形导航
- **pi**: `TreeSelectorComponent` — ASCII 树线 + 展开/折叠 + 过滤 + 水平滚动
- **xylitol**: 实现 `TreeWidget`:
  - 需要 `TreeNode` trait: `fn children(&self) -> &[Self]`
  - 折叠状态在 app state 维护
  - ASCII 树画出 `├─ └─ │`
  - 水平滚动当选中行 anchor 偏移时自动 pan
- **ratatui 积木**: 无原生树 widget,需自建。参考 `ratatui-tree-widget` crate 或自绘。

#### (5) 嵌套设置面板
- **pi**: `SettingsSelectorComponent` — 二值/枚举/子菜单嵌套
- **xylitol**: 实现 `SettingsList<T>`:
  - 每项是一个 `SettingItem<T>`: `{ id, label, description, current_value, update_fn }`
  - 支持 `submenu` — 当选中时切换到子面板,回到父面板
  - 搜索过滤
- **ratatui 积木**: `List` widget 自制

#### (6) 差分渲染
- **pi**: TUI 自动对比前后 `render()` 输出,只写差异
- **xylitol**: ratatui 没有此优化(每次都全量渲染),但对大型输出有帮助。可以自己实现一个 diff buffer layer。
- **优先级**: 低 — ratatui 的全量渲染在 terminal 输出管道下已经很快

#### (7) 状态栏
- **pi**: `FooterComponent` — pwd/git branch/token 统计/上下文%/模型名
- **xylitol**: 直接用 ratatui 的 `Layout` 把界面分为主区域和底部状态栏:
  ```rust
  Layout::vertical([Constraint::Min(1), Constraint::Length(2)])
  ```
  状态栏用 `Paragraph` + `Span` 左/右对齐
- **ratatui 积木**: `Paragraph`,左右对齐通过计算 padding

#### (8) 覆盖层(Overlay)
- **pi**: `ctx.ui.custom(component, { overlay: true })` 在现有画面上叠加渲染一个组件块
- **xylitol**: ratatui 无原生覆盖层。可以用 `Clear` widget 或自定义 `OverlayLayout`:
  ```rust
  fn render_overlay(area: Rect, buf: &mut Buffer, content: impl Widget) {
      let overlay_area = centered_rect(60, 80, area);
      Clear.render(overlay_area, buf);
      content.render(overlay_area, buf);
  }
  ```
  但 ratatui 的 Clear 只清除样式,不保留下层内容。更复杂的方式: 预先渲染下层到 buffer,再在下层之上叠加。
- **优先级**: 中 — `Clear` widget 可以满足 80% 场景

#### (9) 键绑定着色提示
- **pi**: `keyHint(keybinding, "description")` → `dim(keyname) + muted(description)`
- **xylitol**: 简洁实用,直接移植:
  ```rust
  fn key_hint(key: &str, desc: &str) -> Vec<Span> {
      vec![
          Span::styled(key, Style::default().dim()),
          Span::styled(" desc", Style::default().add_modifier(Modifier::DIM)),
      ]
  }
  ```

### 6.3 测试范式可借鉴的

1. **创建真实组件 + fake 依赖**: pi 的测试直接 new 组件,传入最小 fake 对象代替 TUI/回调。xylitol 可以用同样的模式:
   ```rust
   let mut component = ToolExecutionComponent::new(
       "bash",
       args,
       tool_def,
       FakeBackend::new(),
   );
   component.render(area, &mut buf);
   // 断言 buf 内容
   ```
2. **stripAnsi 断言**: pi 用 `stripAnsi()` 去除 ANSI 后做 text containment 断言。ratatui 的 terminal 直接渲染到 `TestBackend` → `buf.get_string(..)` 返回纯文本,更简洁。
3. **状态驱动断言**: 不测试中间 render 调用,只测试给组件输入变化后的输出(pi: `updateResult()` → `render()` → assert)。xylitol 写:
   ```rust
   component.update_result(result);
   component.render(area, &mut buf);
   assert!(buf.get_string(0, 0).contains("expected output"));
   ```
4. **多种展开/折叠状态测试**: pi 覆盖了 collapsed/expanded/紧凑路径等场景,xylitol 也应该覆盖同样的状态变化测试。

### 6.4 中优先级建议

| 建议 | 理由 |
|------|------|
| 搜索型选择器作为通用组件 (generic `SelectableList<T>`) | 80% 的选择器都复用同一模式 |
| 工具渲染 trait (`ToolRenderer`) | 让每个工具自注册渲染器,避免巨大 match |
| 会话树导航 | 高阶功能,但树+折叠+搜索是强大模式 |
| 嵌套设置面板 (submenu pattern) | SettingsList 的子菜单模式优雅 |

### 6.5 低优先级/暂不考虑

| 模式 | 理由 |
|------|------|
| 差分渲染 | ratatui 全量渲染性能足够 |
| inline 图像(Kitty) | 需要 terminal 协议支持,偏离文本核心 |
| OSC 133 标记 | terminal 语义标记,对 xylitol 无直接价值 |
| 多层叠加层栈 | overlay 深度嵌套使用场景少 |
| 异步加载 spinner | 需要异步运行时集成,等 core UI 稳定后再加 |
