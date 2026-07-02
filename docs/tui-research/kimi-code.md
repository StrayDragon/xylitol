# kimi-code TUI 调研报告

## 0. 元信息

- **技术栈**: Ink (React for terminal) + TypeScript, 底层引擎 pi-tui(@moonshot-ai/pi-tui)
- **TUI 根**: `apps/kimi-code/src/tui/`
- **报告日期**: 2026-07-02
- **代码量**: ~3k 行主协调器 + 30+ dialogs + 10 controllers + 30+ utilities
- **调研版本**: 最新 main 分支

---

## 1. 组件全景清单

### 1.1 chrome/ — 常驻外壳

| 组件名 | 类别 | 文件 | 职责 | 对 xylitol 可借鉴度 |
|---|---|---|---|---|
| BannerComponent | chrome | `banner.ts` | 欢迎面板下方 banner 通知,once/cooldown/always 三种展示模式 | ★★ 通知栏模式 |
| FooterComponent | chrome | `footer.ts` | 底部状态栏:model/模式/会话信息/瞬时提示 | ★★★ 常驻 status bar |
| DeviceCodeBoxComponent | chrome | `device-code-box.ts` | 设备授权码展示框(URL+code) | ★ 特定场景 |
| GutterContainer | chrome | `gutter-container.ts` | 左右边距包装容器,统一页边距 | ★★★ 布局 wrapper |
| MoonLoader | chrome | `moon-loader.ts` | 旋转动画 spinner(braille/moon 两种) | ★★ 加载指示 |
| TodoPanelComponent | chrome | `todo-panel.ts` | 会话 todo 列表面板(可折叠/展开) | ★★ 任务面板 |
| WelcomeComponent | chrome | `welcome.ts` | 空会话欢迎界面 | ★★ 启动页 |
| WorkingTips | chrome | `working-tips.ts` | 等待提示语随机抽取 | ★ 贴心细节 |

### 1.2 dialogs/ — Overlay 对话框体系(30+ 个,核心章节)

| 组件名 | 分类 | 文件 | 职责 | 借鉴度 |
|---|---|---|---|---|
| ApprovalPanelComponent | **Approval** | `approval-panel.ts` | 工具调用审批:diff/文件/shell/搜索等展示+选项 | ★★★★★ |
| ApprovalPreviewViewer | **Viewer** | `approval-preview.ts` | 全屏预览审批中的 diff/文件内容 | ★★★ |
| QuestionDialogComponent | **Question** | `question-dialog.ts` | 多问题采集:单选/多选/Other 输入/Review 提交 | ★★★★★ |
| ModelSelectorComponent | **Selector** | `model-selector.ts` | 模型选择:搜索+列表+thinking 三态 | ★★★★ |
| TabbedModelSelectorComponent | **Selector** | `tabbed-model-selector.ts` | 分层模型选择器:provider 标签页切换 | ★★★★ |
| SessionPickerComponent | **Selector** | `session-picker.ts` | 会话浏览/切换:CWD/全量范围 | ★★★ |
| EffortSelectorComponent | **Selector** | `effort-selector.ts` | thinking effort 选择 | ★★ |
| PermissionSelectorComponent | **Selector** | `permission-selector.ts` | permission 模式选择 | ★★ |
| ThemeSelectorComponent | **Selector** | `theme-selector.ts` | 主题选择 | ★★ |
| SettingsSelectorComponent | **Selector** | `settings-selector.ts` | TUI 设置选择面板 | ★★ |
| PluginsSelectorComponent | **Selector** | `plugins-selector.ts` | 插件管理(30KB,最复杂的 selector) | ★★★★ |
| ProviderManagerComponent | **Selector** | `provider-manager.ts` | provider 增删管理 | ★★★ |
| ExperimentsSelectorComponent | **Selector** | `experiments-selector.ts` | 实验特性开关 | ★★ |
| ChoicePickerComponent | **Selector** | `choice-picker.ts` | 通用选项选择器 | ★★★ |
| HelpPanelComponent | **Viewer** | `help-panel.ts` | /help 快捷键+命令显示 | ★★ |
| TaskOutputViewerComponent | **Viewer** | `task-output-viewer.ts` | 任务输出全屏查看器 | ★★★ |
| TasksBrowserComponent | **Viewer** | `tasks-browser.ts` | 后台任务浏览(22KB) | ★★★ |
| CompactionComponent | **Viewer** | `compaction.ts` | 上下文压缩状态展示 | ★★ |
| GoalQueueManagerComponent | **Viewer** | `goal-queue-manager.ts` | 目标队列管理(20KB) | ★★★ |
| APIKeyInputDialogComponent | **Input** | `api-key-input-dialog.ts` | API key 输入 | ★★ |
| FeedbackInputDialogComponent | **Input** | `feedback-input-dialog.ts` | 反馈输入框 | ★★ |
| CustomRegistryImportComponent | **Input** | `custom-registry-import.ts` | 自定义 registry 导入(带 URL/Token 输入) | ★★ |
| EditorSelectorComponent | **Selector** | `editor-selector.ts` | 外部编辑器选择 | ★ |
| PlatformSelectorComponent | **Selector** | `platform-selector.ts` | 平台选择 | ★ |
| StartPermissionPromptComponent | **Approval** | `start-permission-prompt.ts` | 启动权限提示 | ★★ |
| GoalStartPermissionPromptComponent | **Approval** | `goal-start-permission-prompt.ts` | 目标启动权限提示 | ★★ |
| SwarmStartPermissionPromptComponent | **Approval** | `swarm-start-permission-prompt.ts` | swarm 启动权限提示 | ★★ |
| UndoSelectorComponent | **Selector** | `undo-selector.ts` | undo 操作选择 | ★★ |
| UpdatePreferenceSelectorComponent | **Selector** | `update-preference-selector.ts` | 更新偏好选择 | ★ |

### 1.3 editor/ — 输入编辑

| 组件名 | 文件 | 职责 | 借鉴度 |
|---|---|---|---|
| CustomEditor | `custom-editor.ts` | 自定义输入框:多行/slash 自动补全/历史/边框高亮 | ★★★ |
| FileMentionProvider | `file-mention-provider.ts` | slash 命令自动补全+文件路径补全 | ★★★ |
| WrappingSelectList | `wrapping-select-list.ts` | 自动换行选择列表 | ★★ |

### 1.4 media/ — 富内容渲染

| 组件名 | 文件 | 职责 | 借鉴度 |
|---|---|---|---|
| CodeHighlight | `code-highlight.ts` | 代码语法高亮(cli-highlight) | ★★★★ |
| DiffPreview | `diff-preview.ts` | diff 渲染:聚类+行号+ANSI 着色+展开 | ★★★★★ |
| ImageThumbnail | `image-thumbnail.ts` | Kitty 协议内嵌图片缩略图 | ★ |

### 1.5 messages/ — 消息渲染(22 个)

| 组件名 | 文件 | 职责 | 借鉴度 |
|---|---|---|---|
| UserMessageComponent | `user-message.ts` | 用户消息 | ★★ |
| AssistantMessageComponent | `assistant-message.ts` | 助手消息(markdown 渲染) | ★★★ |
| ThinkingComponent | `thinking.ts` | 思考过程折叠 | ★★ |
| ToolCallComponent | `tool-call.ts` | 工具调用卡片 | ★★★ |
| ShellRunComponent | `shell-run.ts` | `!` shell 命令实时输出 | ★★★ |
| ShellExecutionComponent | `shell-execution.ts` | shell 执行详情 | ★★ |
| StatusMessageComponent | `status-message.ts` | 状态消息 | ★★ |
| NoticeMessageComponent | `status-message.ts` | 通知消息 | ★ |
| PlanBoxComponent | `plan-box.ts` | 计划框 | ★★ |
| AgentGroupComponent | `agent-group.ts` | 子 agent 组 | ★★ |
| 更多... | `cron-message/goal-panel/...` | 专用消息类型 | ★ |

### 1.6 panes/ — 活动面板

| 组件名 | 文件 | 职责 | 借鉴度 |
|---|---|---|---|
| ActivityPaneComponent | `activity-pane.ts` | 活动态面板(waiting/thinking/composing/tool) | ★★★ |
| QueuePaneComponent | `queue-pane.ts` | 消息队列展示 | ★★★ |
| BtwPanelComponent | `btw-panel.ts` | 分叉侧问面板 | ★★ |

---

## 2. Overlay/Dialog 体系(核心章节)

### 2.1 dialogs 分类

kimi-code 的 30+ 个 dialogs 可清晰分为四大类:

#### (A) Approval 类(工具审批)
覆盖 agent 调用工具前的审批流程。**核心交互:展示审批内容 → 用户选择选项 → 异步回传结果**

| 组件 | 触发场景 | 选项形态 |
|---|---|---|
| **ApprovalPanelComponent** | 通用:WriteFile/Bash/Edit/UrlFetch/Search 等 | 多选项(批准一次/批准会话/拒绝/带反馈拒绝) |
| StartPermissionPromptComponent | 首次任务启动 | 简化版 |
| GoalStartPermissionPromptComponent | 目标启动 | 简化版 |
| SwarmStartPermissionPromptComponent | swarm 启动 | 简化版 |

**ApprovalPanel 典型生命周期:**
1. Agent 发起工具调用 → SDK `approval_handler` 触发
2. `createApprovalRequestHandler` → `ApprovalController.show(payload)`
3. `modalCoordinator.showApproval(data)` → `showApprovalPanel`
4. `KimiTUI.showApprovalPanel`: 创建 `ApprovalPanelComponent`,调用 `mountEditorReplacement(panel)` → 替换编辑器
5. 用户选择(↑/↓/数字键/Enter/Esc)
6. 回调 `onResponse` → `ApprovalController.respond(response)`
7. `KimiTUI.hideApprovalPanel`: `restoreEditor()` → 恢复编辑器
8. 审批结果追加为 transcript entry

**关键数据契约** - `PendingApproval`:
```typescript
interface ApprovalPanelData {
  id: string;
  tool_call_id: string;
  tool_name: string;       // "Bash" | "WriteFile" | "Edit" | ...
  action: string;           // 动作描述
  description: string;      // 文字描述
  display: DisplayBlock[];  // 展示块数组(见下方)
  choices: ApprovalPanelChoice[];  // 选项数组
}
```

**DisplayBlock 类型系统**(11 种展示块):
```typescript
type DisplayBlock =
  | BriefDisplayBlock      // 纯文字说明
  | DiffDisplayBlock       // diff {path, old_text, new_text, old_start, new_start}
  | ShellDisplayBlock      // 命令  {command, cwd, danger, description}
  | FileOpDisplayBlock     // 文件操作 {operation: 'read'|'write'|'edit'|'glob'|'grep', path}
  | FileContentDisplayBlock // 文件内容 {path, content, language}
  | UrlFetchDisplayBlock   // URL 请求 {url, method}
  | SearchDisplayBlock     // 搜索 {query, scope}
  | InvocationDisplayBlock // 调用 {kind: 'agent'|'skill', name}
  | TodoDisplayBlock       // TODO 列表 {items: {title, status}[]}
  | BackgroundTaskDisplayBlock // 后台任务
  | ...;
```

#### (B) Question 类(多问题采集)
**核心交互:逐个回答多问题(单选/多选/Other输入) → Review 页 → 提交**
对应 SDK `question_handler`。典型一次提 3-6 个问题。

**QuestionDialogComponent 内部 tab 结构:**
```
[Q1] [Q2] [Q3] [Review]    ← tab 切换
──────────────────────────
▼ Question 1 title
  ❯ Option A
    Option B ← current
    Option C
──────────────────────────
```

每个问题支持: `multi_select` 表示多选,`other_label` 表示可输入 Other。最后 Review tab 汇总所有答案,显示未答警告,提交前有确认。

**Question 的 5 步交互闭环**:
1. Agent 发起问题 → `QuestionController.show(payload)`
2. `modalCoordinator.showQuestion(data)` → `showQuestionDialog`
3. 用户逐 tab 回答问题(↑/↓ 选择,Space 多选,打字输入 Other)
4. 切换到 Review tab,确认提交 → `onAnswer(response)`
5. 关闭 dialog,恢复编辑器

#### (C) Selector 类(列表选择器)
**核心模式:弹出列表 → 搜索/浏览/选择 → 关闭并应用**

这是数量最多的一类,共享统一的**布局规范**(来自 DESIGN.md):

```
────────────────────────────────────────  ① 顶部边框(primary,整宽 ─)
 Select a model  (type to search)         ② 标题(primary+bold)+(可搜索时后缀 textMuted)
 ↑↓ navigate · Enter select · Esc cancel  ③ hint(textMuted,整行,无键位高亮)
                                           ④ 空行
 Search: gpt                               ⑤ 搜索行(仅在有 query 时出现)
  ❯ GPT-5            openai                ⑥ 列表项:指针+名称(左)+次要列(右,textMuted)
    Kimi K2          Kimi Code ← current      当前项行尾 ` ← current`(success)
                                           ⑦ 空行
 ▼ 3 more                                  ⑧ 滚动/匹配指示
────────────────────────────────────────  ⑨ 底部边框(primary,整宽 ─)
```

**Selector 共有行为:**
- 所有 selectors 复用一个 **`SearchableList`** 工具类(光标/搜索/翻页状态机)
- 统一键位: `↑↓` 移动, `PgUp/PgDn` 翻页, `Enter` 确认, `Esc` 取消(Esc 两段式:先清 query 再关闭)
- 统一符号: `SELECT_POINTER`("❯"), `CURRENT_MARK`(" ← current")
- 颜色全部来自 `colors.<token>`, 禁用 named color
- 每行经 `truncateToWidth(line, width)`

**Selector 子类:**
- **ModelSelectorComponent**: 模型列表+provider 右列+thinking 三态控件
- **TabbedModelSelectorComponent**: ModelSelector 外层包 tab 条(All/Kimi/openai/...),Tab 切换
- **PluginsSelectorComponent**: 开关列表(Space toggle),每行状态标签( enabled/ disabled),次要信息行
- **SessionPickerComponent**: 支持范围切换(CWD↔All)
- **ThemeSelector/EffortSelector/PermissionSelector/SettingsSelector**: 简化选择器
- **ChoicePickerComponent**: 通用选项选择器,可外部调用

#### (D) Viewer 类(全屏查看器)
**核心模式:临时替换整个 UI 的子组件树 → 展示全屏内容 → 关闭恢复**

| 组件 | 触发 | 实现方式 |
|---|---|---|
| **ApprovalPreviewViewer** | 审批面板按 `ctrl+e` | 保存 `ui.children` → 清空 → 添加 viewer → 关闭时恢复 |
| **TaskOutputViewerComponent** | `/tasks` 或任务详情 | 同上,替换整个 editor 容器 |
| **HelpPanelComponent** | `/help` | `mountEditorReplacement` |
| **TasksBrowserComponent** | `/tasks` | `editorKeyboard` 挂载 |

### 2.2 Overlay 叠加机制

kimi-code **并非**用图层/堆叠来叠加 overlay,而是用**替换模式**:

#### 核心机制: `mountEditorReplacement + restoreEditor`

```typescript
// KimiTUI 中的挂载/恢复:
mountEditorReplacement(panel: Component & Focusable): void {
  this.state.editorContainer.clear();      // 清空编辑器容器
  this.state.editorContainer.addChild(panel); // 放入 overlay 组件
  this.state.ui.setFocus(panel);           // 焦点转到 overlay
  this.state.ui.requestRender();
}

restoreEditor(): void {
  this.state.editorContainer.clear();       // 移除 overlay
  this.state.editorContainer.addChild(this.state.editor); // 恢复编辑器
  this.state.ui.setFocus(this.state.editor);
  this.state.ui.requestRender();
}
```

**关键架构决策:所有 overlay 都插入到 `editorContainer` 位置(终端最下方),替换输入框。** 而不是叠加在 transcript 区域上方。

#### ApprovalPreview 的"全屏替换"模式(更激进的层级替换)

```typescript
// 保存整个 UI 的 children,用 viewer 完全替换
private openApprovalPreview(panel, block): void {
  const savedChildren = [...this.state.ui.children];  // 快照所有子组件
  const viewer = new ApprovalPreviewViewer(...);
  this.state.ui.clear();
  this.state.ui.addChild(viewer);  // 替换为全屏 viewer
  this.state.ui.setFocus(viewer);
  this.approvalPreview = { component: viewer, savedChildren, panel };
}

private closeApprovalPreview(): void {
  this.state.ui.clear();
  for (const child of preview.savedChildren) {
    this.state.ui.addChild(child);  // 恢复原组件树
  }
  this.state.ui.setFocus(preview.panel);  // 焦点回到原审批面板
}
```

#### 焦点管理规则

| 操作 | 焦点移到 | 实现 |
|---|---|---|
| overlay 打开 | overlay 组件 | `this.state.ui.setFocus(panel)` |
| Esc/Enter 关闭 | 编辑器 | `this.state.ui.setFocus(this.state.editor)` |
| ctrl+e 全屏预览 | 预览 viewer | `this.state.ui.setFocus(viewer)` |
| 预览关闭 | 原审批 panel | `this.state.ui.setFocus(preview.panel)` |

### 2.3 触发与退出流程

#### Approval 交互闭环:

```
Agent 发起工具调用
  ↓
SDK approval_handler(ApprovalRequest)
  ↓
createApprovalRequestHandler
  → adaptApprovalRequest  (将 SDK 类型转成 UI 类型)
  → ApprovalController.show(ApprovalPanelData)
    → 如有当前面板,排队(RpcModalCoordinator)
    → 无当前面板:
      showApprovalPanel(data)
        → patchLivePane({ pendingApproval })
        → new ApprovalPanelComponent(data, onResponse, onToggle, onPreview)
        → mountEditorReplacement(panel)
  ↓
用户交互:
  ↑/↓ 移动选择
  1-9 数字键快捷选
  Enter 确认(带 feedback 则进入输入模式)
  Esc/Ctrl-C/Ctrl-D → rejected
  ctrl+e → 全屏 diff 预览
  ctrl+o → 切换工具输出展开
  ↓
用户选择 → onResponse(response)
  → ApprovalController.respond(response)
    → 同类 queue 自动决议(session-scope 自动批准后续)
    → hideApprovalPanel
      → patchLivePane({ pendingApproval: null })
      → restoreEditor()
  → appendApprovalTranscriptEntry (在 transcript 追加审批记录)
  ↓
SDK approval_handler 返回 ApprovalResponse
  → Agent 继续执行或被拒绝
```

#### Question 交互闭环:

```
Agent 发起问题 → SDK question_handler
  ↓
createQuestionAskHandler
  → QuestionController.show(QuestionPanelData)
    → showQuestionDialog(data)
      → patchLivePane({ pendingQuestion })
      → new QuestionDialogComponent(data, onAnswer, maxVisible, onToggle)
      → mountEditorReplacement(dialog)
  ↓
用户交互:
  Tab 切换问题(各问题独立 tab)
  ↑/↓ 选择选项, Space 多选(多选模式)
  Other 文本输入
  Review tab 汇总确认
  ↓
用户提交 → QuestionController.respond(response)
  → hideQuestionDialog → restoreEditor()
```

#### Selector 交互闭环:

```
用户输入 /model 或 /permission 等 slash 命令
  ↓
dispatch → handleModelCommand/showPermissionPicker
  ↓
创建 Selector 组件 → mountEditorReplacement(selector)
  ↓
用户选择 → onSelect(value) 回调
  → 执行逻辑(设模型/切模式)
  → restoreEditor()
  ↓
或 Esc → onCancel() → restoreEditor()
```

---

## 3. 命令分发(对标 xylitol c335)

### 3.1 dispatch.ts 路由机制

kimi-code 的命令分发分为四个层次:

#### 层次 1: 输入入口 — `KimiTUI.handleUserInput`

```typescript
handleUserInput(text: string): void {
  // 1. bash 模式处理
  if (wasBashMode) { runShellCommandFromInput(text); return; }

  // 2. 统一分发
  slashCommands.dispatchInput(this, text);
}
```

#### 层次 2: 命令识别 — `dispatchInput`

```typescript
export function dispatchInput(host: SlashCommandHost, text: string): void {
  if (parseSlashInput(text) !== null) {
    // 以 / 开头 → 执行 slash 命令
    void executeSlashCommand(host, text);
    return;
  }
  // 普通文本 → 发送给 agent
  host.sendNormalUserInput(text);
}
```

`parseSlashInput`: 解析 `/command args` → `{name, args}`。排除文件路径(`/usr/local/bin`),允许 namespaced 插件命令(`plugin:frontend/component`)。

#### 层次 3: 命令路由 — `executeSlashCommand`

```typescript
async function executeSlashCommand(host, input) {
  const parsedCommand = parseSlashInput(input);
  const intent = resolveSlashCommandInput({...});

  switch (intent.kind) {
    case 'not-command': return;
    case 'blocked':     // 忙碌状态无法执行
    case 'invalid':     // 未知命令
    case 'skill':       // skill 技能命令
    case 'plugin-command': // 插件命令
    case 'message':     // 传给模型作为普通消息
    case 'builtin':     // 内置命令 → handleBuiltInSlashCommand
  }
}
```

`resolveSlashCommandInput` 做了**五层解析**:
1. parseSlashInput → 抽取 `/name args`
2. `findBuiltInSlashCommand(name)` → 匹配内置命令注册表
3. 若内置命中 → 检查 `availability`(idle-only 时 streaming/compacting 会 blocked)
4. 若未命中 → 查 `skillCommandMap` → 匹配 skill
5. 若未命中 → 查 `pluginCommandMap` → 匹配插件命令
6. 全未命中 → 作为 `/dance` 彩蛋候选 → 作为普通消息传给 agent

#### 层次 4: 内置命令执行 — `handleBuiltInSlashCommand`

**30+ `case` 的巨型 switch**,每个 case 调用对应的 handle 函数:

```typescript
async function handleBuiltInSlashCommand(host, name, args) {
  switch (name) {
    case 'exit':    void host.stop(); return;
    case 'help':    host.showHelpPanel(); return;
    case 'model':   await handleModelCommand(host, args); return;
    case 'provider': await handleProviderCommand(host); return;
    case 'undo':    await handleUndoCommand(host, args); return;
    // ... 30+ 个 case
  }
}
```

### 3.2 命令→session 执行→UI 反映的全链路

以 `/model` 为例:

```
用户输入 "/model gpt-4"
  ↓
handleUserInput("/model gpt-4")
  ↓
dispatchInput(host, "/model gpt-4")
  ↓
parseSlashInput → {name: "model", args: "gpt-4"}
  ↓
resolveSlashCommandInput → {kind: "builtin", name: "model"}
  ↓
handleBuiltInSlashCommand(host, "model", "gpt-4")
  ↓
handleModelCommand(host, "gpt-4")  ← 在 commands/config.ts 中
  │
  ├─ args 为空 → 弹出 ModelSelectorComponent
  │   → mountEditorReplacement(selector)
  │   → 用户选模型 → onSelect(selection)
  │   → session.setModel(selection.alias)
  │   → session.setThinkingEffort(selection.thinking)
  │   → syncRuntimeState(session)
  │   → showStatus("Switched to Kimi K2.")
  │   → restoreEditor()
  │
  └─ args 非空 → 直接设置
      → session.setModel("gpt-4")
      → syncRuntimeState(session)
      → showStatus("Switched to gpt-4.")
```

### 3.3 与 xylitol protocol::Command dispatch 的异同

| 维度 | kimi-code | xylitol(规划) |
|---|---|---|
| **输入触发** | 文本输入框 `/cmd args` | 可能是 `:` 命令模式 |
| **解析** | `parseSlashInput` 纯字符串解析 | `protocol::Command` 枚举+args |
| **路由** | `resolveSlashCommandInput` 五层解析树 | 理想:Command enum → match |
| **执行** | 每个 `handleXxx` 函数带 `SlashCommandHost` | `CommandHandler` trait |
| **host 接口** | `SlashCommandHost` interface(40+ 方法) | 对应 `AppHandle` 或 `Context` |
| **与 session 对接** | 直接调用 `session.setXxx()` | 通过 `Event` 总线 |
| **UI 反映** | 直接 `host.setAppState()/showStatus()` | Event→State update→rerender |
| **异步** | 全部 `async`,await | 同 |
| **可用性检查** | `availability: 'idle-only' | 'always'` + 运行时 busy check | 需要类似机制 |
| **命令注册** | 静态 `BUILTIN_SLASH_COMMANDS` 数组 | 类似 `registry` |
| **动态命令** | skill 命令+plugin 命令动态构建 | 可能需要相同机制 |

**kimi-code的优势:**
- `SlashCommandHost` 接口定义清晰,所有命令共享一套 host 方法
- 命令注册表集中管理(aliases/description/priority/availability/completeArgs)
- 动态命令(skill/plugin)的自动注册和补全

**kimi-code的冗余:**
- 30+ case 的手动 switch(没有用 registry 自动迭代)
- `SlashCommandHost` 膨胀到 40+ 方法(但这是必然的,因为命令需要多种能力)

---

## 4. 富内容渲染

### 4.1 DiffPreview

**文件**: `components/media/diff-preview.ts` (~280 行)

**算法**: 标准 LCS(最长公共子序列) diff + 聚类(clustered)展示

**核心数据结构**:
```typescript
interface DiffLine {
  kind: 'context' | 'add' | 'delete';
  lineNum: number;
  code: string;
}
```

**配色方案**(ANSI 颜色 token):
```typescript
const palette = currentTheme.palette;
{
  add:   chalk.hex(palette.diffAdded),        // 新增行底色
  del:   chalk.hex(palette.diffRemoved),      // 删除行底色
  addBold: chalk.hex(palette.diffAddedStrong),// 新增高亮
  delBold: chalk.hex(palette.diffRemovedStrong),// 删除高亮
  gutter: chalk.hex(palette.diffGutter),      // 行号区
  meta: chalk.hex(palette.diffMeta),          // 元信息
}
```

**聚类展示函数** `renderDiffLinesClustered(oldText, newText, path, options)`:
- 计算全量 diff lines
- 以 `contextLines`(默认 3)聚类上下文分组
- 组间用 `···` 省略号连接
- 超 `maxLines` 截断+显示 "ctrl+e to preview" 提示
- 每组顶部有文件路径元信息

### 4.2 CodeHighlight

**文件**: `components/media/code-highlight.ts` (~60 行)

**技术**: `cli-highlight` 库(supportsLanguage/highlight)

**语言映射**:
```typescript
const EXT_LANG_MAP = {
  ts: 'typescript', tsx: 'typescript',
  py: 'python', rs: 'rust', go: 'go',
  sh: 'bash', json: 'json', md: 'markdown',
  c: 'c', cpp: 'cpp', ...
};
```

**暴露函数**:
```typescript
function langFromPath(filePath: string): string | undefined  // 根据扩展名映射语言
function highlightLines(code: string, lang: string | undefined): string[]  // 高亮后分行
```

### 4.3 Media 数据契约

**富内容数据全部走 `DisplayBlock` 联合类型**(见 2.1 节 A 类)。

每个 DisplayBlock 的 `type` 区分渲染方式:
- `diff` → `computeDiffLines` + `renderDiffLinesClustered`
- `file_content` → `highlightLines` + 行号格式化
- `shell` → 命令块渲染(危险标志/工作目录)
- `url_fetch` / `search` / `file_op` / `invocation` → 不同格式的摘要行

**无 HTML/JSX 渲染**,全部输出 ANSI 字符串行。这直接适用 ratatui 的 `Text`/`Span`。

---

## 5. 测试模式

### 5.1 测试文件结构

```
test/tui/
├── banner/
├── commands/          ← parse/dispatch 单元测试
├── components/
│   ├── chrome/
│   ├── dialogs/       ← 16 个 dialog 测试文件(覆盖主要组件)
│   ├── editor/
│   ├── media/         ← diff/code-highlight/image
│   ├── messages/      ← 消息组件
│   │   └── tool-renderers/
│   ├── panels/
│   └── panes/
├── constant/
├── controllers/
├── easter-eggs/
├── input/
├── reverse-rpc/       ← approval/question controller
├── theme/
└── utils/
```

### 5.2 组件测试做法

**使用 vitest + 快照 + 行为测试**。典型模式:

```typescript
import { ApprovalPanelComponent } from '#/tui/components/dialogs/approval-panel';
import { describe, expect, it } from 'vitest';

function strip(text: string): string {
  return text.replaceAll(/\u001B\[[0-9;]*m/g, '');  // 去 ANSI 码
}

// 1. 构造 fake 数据
function makePending(): PendingApproval {
  return {
    data: {
      id: 'approval_1', tool_call_id: 'tool_1',
      tool_name: 'WriteFile', action: 'write a file',
      description: 'Update README.md',
      display: [],
      choices: [
        { label: 'Approve once', response: 'approved' },
        { label: 'Approve for session', response: 'approved_for_session' },
        { label: 'Reject', response: 'rejected' },
        { label: 'Reject with feedback', response: 'rejected', requires_feedback: true },
      ],
    },
  };
}

// 2. 渲染组件并断言输出
describe('ApprovalPanelComponent', () => {
  it('renders hint with numeric shortcuts', () => {
    const dialog = new ApprovalPanelComponent(makePending(), vi.fn());
    const out = strip(dialog.render(80).join('\n'));
    expect(out).toContain('1/2/3/4 choose');
  });

  it('rejects on Escape', () => {
    const { dialog, responses } = makeDialog();
    dialog.handleInput('\x1b');  // Escape
    expect(responses[0]!.response).toBe('rejected');
  });
});
```

**ModelSelector 测试更精细**,使用常量 `UP = ESC + '[A'` 模拟按键,断言渲染输出中的行匹配:

```typescript
it('moves selection down', () => {
  const picker = new ModelSelectorComponent({...});
  picker.handleInput(DOWN);
  const out = text(picker);
  expect(out).toMatch(/❯ Kimi K2/);
});
```

### 5.3 测试数据构造

- 用工厂函数 (如 `makePending()`, `model()` ) 构造 fake 数据
- `PendingApproval` / `PendingQuestion` 等类型直接构造对象
- 与生产环境共用类型定义(file `reverse-rpc/types.ts`)

---

## 6. 可视化示意(TUI 字符界面)

### 6.1 主界面 + ApprovalPanel overlay

```
┌──────────────────────────────────────────────────────────────┐
│   ╭──────────────────────────────────────────────────────╮   │
│   │  Kimi K2 is thinking...                              │   │  ← transcript 区域
│   │                                                      │   │
│   │  ❯  I'll update the README with the new API docs.   │   │
│   │                                                      │   │
│   │  ╭─ Write /README.md ───────────────────────────╮    │   │
│   │  │ +## API Reference                            │    │   │  ← tool call 卡片
│   │  │ +### POST /api/v1/users                      │    │   │
│   │  │ +```                                        │    │   │
│   │  ╰──────────────────────────────────────────────╯    │   │
│   ╰──────────────────────────────────────────────────────╯   │
│                                                              │
│  ──────────────────────────────────────────────────────────  │  ← approval panel
│    ▶ Run this command?                                       │     (替换了编辑器区域)
│                                                              │
│      curl -X POST https://api.example.com/deploy             │
│                                                              │
│    1. Approve once                                            │
│    2. Approve for session                                     │
│  ❯ 3. Reject                                                  │  ← 选中项 ❯ + primary bold
│    4. Reject with feedback                                    │
│                                                              │
│    ↑/↓ select · 1/2/3/4 choose · ↵ confirm                  │
│  ──────────────────────────────────────────────────────────  │
│                                                              │
│  Kimi K2 · plan · manual · 1.2k/200k  [Queue: 2]            │  ← footer 状态栏
└──────────────────────────────────────────────────────────────┘
```

**组件标注**: `ApprovalPanelComponent` 替换了 `editorContainer`(输入框区域)。transcript 区域保持原样,activity pane 隐藏(pendingApproval 非 null → mode='hidden')。

### 6.2 TabbedModelSelector 分层选择器

```
┌──────────────────────────────────────────────────────────────┐
│  ──────────────────────────────────────────────────────────  │  ← 顶部边框
│   Select a model  (type to search)                           │  ← 标题
│   Tab toggle provider · ↑↓ navigate · Enter select · Esc cancel│  ← hint(textMuted)
│                                                              │  ← 空行
│    All    Kimi Code    openai                                │  ← tab 条(激活填充背景)
│                                                              │  ← 空行
│    Search: kimi                                              │  ← 搜索框
│                                                              │
│  ❯  Kimi K2          Kimi Code        ← current             │  ← 当前模型
│     Kimi K1.5         Kimi Code                              │  ← 普通选项
│     Kimi Thinking     Kimi Code                              │
│                                                              │
│    Thinking  [ On ]  Off  (←→ to switch)                    │  ← thinking 三态控件
│                                                              │
│  ──────────────────────────────────────────────────────────  │  ← 底部边框
│                                                              │
│  Kimi K2 · plan · manual · 1.2k/200k                        │  ← footer
└──────────────────────────────────────────────────────────────┘
```

**组件标注**: `TabbedModelSelectorComponent` 内含多个 `ModelSelectorComponent`(每个 tab 一个)。tab 条使用 `renderTabStrip` 工具函数。

### 6.3 DiffPreview 展示

```
┌──────────────────────────────────────────────────────────────┐
│  ── src/utils/format.ts ────────────────────────────────     │  ← 文件路径 + 边框
│                                                              │
│     1  import { join } from 'pathe';                        │  ← 上下文行
│     2                                                       │
│  ❰  3  function formatPath(path: string) {                  │  ← 删除行(红色底色)
│  ❱   3  function formatPathNormalized(path: string) {       │  ← 新增行(绿色底色)
│     4    const home = process.env['HOME'] ?? '';            │
│     5    if (home && path.startsWith(home)) {               │
│     6  ❰   return '~' + path.slice(home.length);           │  ← 删除行内高亮
│     6  ❱   return '~/' + relative(home, path);              │  ← 新增行内高亮
│     7    }                                                   │
│     8    return path;                                        │
│     9  }                                                     │
│                                                              │
│     ···                                                      │  ← 聚类省略
│                                                              │
│    12  export { formatPath };                                │
│                                                              │
│  ──────────────────────────────────────────────────────────  │
│  ctrl+e to preview full diff                                 │  ← 可扩展提示
└──────────────────────────────────────────────────────────────┘
```

**组件标注**: `renderDiffLinesClustered` 输出 ANSI 格式化行。配色: `diffAdded` / `diffRemoved` 底色, `diffGutter` 行号区颜色, `diffMeta` 元信息。

---

## 7. 对 xylitol 的启示

### 7.1 Overlay 体系在 ratatui 中的实现方案

kimi-code 使用 `mountEditorReplacement / restoreEditor` 模式,本质是**组件替换**而非图层叠加。在 ratatui 中对应的可行方案:

| kimi-code 做法 | ratatui 等效 | 说明 |
|---|---|---|
| `mountEditorReplacement(panel)` | 切换 `AppState::active_overlay` + 条件渲染 `Overlay` widget | 状态驱动,flexbox/constraint 控制位置 |
| `restoreEditor()` | 切换回 `AppState::active_overlay = None` | 恢复常规布局 |
| `editorContainer.clear()` + `addChild(panel)` | `if let Some(overlay) = &state.overlay { render_overlay(...) }` | 条件分支控制 |
| 全屏 preview(`savedChildren`快照) | `Area::default()` + `Clear` widget | `Clear` 擦除下层,然后渲染全屏内容 |
| `state.ui.setFocus()` | 焦点状态机 | ratatui `Focus` 机制或自定义 `focused` 枚举 |

**推荐实现:**
```rust
// 状态驱动 overlay
enum ActiveOverlay {
    None,
    Approval(ApprovalState),
    Question(QuestionState),
    Selector(Box<dyn Selector>),
    Viewer(ViewerKind),
}

// 渲染时:
match &state.active_overlay {
    None => render_normal(),
    Some(overlay) => {
        // 使用 Clear 擦除下层
        let overlay_area = centered_rect(60, 80, frame.area());
        frame.render_widget(Clear, overlay_area);
        render_overlay(overlay, overlay_area);
    }
}
```

**关键要点:**
- **不要多图层叠加,用清除+渲染代替**。ratatui 没有真正的 z-order。
- **Overlay 的焦点管理**: 用枚举 `FocusTarget` 控制键盘事件分发。
- **全屏 viewer**: 直接用 `Clear` + 全屏 area 渲染,无需保存/恢复子组件树。
- **队列管理**: kimi-code 的 `ReverseRpcModalCoordinator` 的模式直接可用。

### 7.2 命令分发范式对 c335 的启示

kimi-code 的 **`dispatchInput → resolveSlashCommandInput → handleBuiltInSlashCommand`** 三层结构值得 xylitol 借鉴:

**推荐 c335 实现:**
```rust
// 1. 命令注册表(对标 BUILTIN_SLASH_COMMANDS)
#[derive(Clone)]
struct CommandDef {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    availability: CommandAvailability,
    handler: fn(&mut AppContext, args: &str) -> Result<()>,
}

// 2. 解析层
enum CommandIntent {
    Builtin { name: String, args: String },
    Plugin { name: String, args: String },
    Skill { name: String, args: String },
    Message(String), // 传给 agent
    Blocked { reason: BusyReason },
}

// 3. Host 接口(对标 SlashCommandHost)
trait CommandHost {
    fn state(&self) -> &AppState;
    fn session(&self) -> Option<&Session>;
    fn set_state(&mut self, patch: StatePatch);
    fn show_error(&mut self, msg: String);
    fn show_status(&mut self, msg: String, color: Color);
    fn mount_overlay(&mut self, overlay: ActiveOverlay);
    fn restore_editor(&mut self);
    // ...
}
```

**kimi-code 的可借鉴点:**
- **`availability` 机制**: idle-only 命令在 streaming/compacting 时自动 blocked,避免误操作
- **`completeArgs` 回调**: 命令参数自动补全,提升用户体验
- **动态命令注入**: skill 和 plugin 命令动态注册,无需改主路由
- **`priority` 排序**: 命令列表按优先级排序,常用命令排前面
- **host 接口隔离**: 命令不直接操作 TUI 内部,通过 host 方法间接操作

### 7.3 富内容(diff/code)渲染借鉴

**diff 渲染可直接移植**:
- LCS diff 算法(语言无关)
- 聚类展示算法(组间 `···` 省略)
- ANSI 颜色 token 系统(ratatui 用 `Style` 代替)

```rust
// ratatui diff line
struct DiffLine {
    kind: DiffLineKind, // Context | Addition | Deletion
    line_num: usize,
    text: String,
}

// 渲染:
for line in &clustered_lines {
    let style = match line.kind {
        DiffLineKind::Addition => Style::new().bg(Color::Rgb(22, 55, 33)),
        DiffLineKind::Deletion => Style::new().bg(Color::Rgb(55, 22, 22)),
        DiffLineKind::Context => Style::default(),
    };
    line_spans.push(Span::styled(&line.text, style));
}
```

**code highlight**: ratatui 可直接用 `syntect` 库(更强大),或简化的行号+ANSI。

### 7.4 风险/陷阱

1. **Ink 的 Virtual DOM vs ratatui 即时渲染**:
   - kimi-code 利用 React 的 diffing,组件可局部更新
   - ratatui 每帧全量重绘,必须小心:
     - 动画(spinner)需要帧循环
     - 大 diff 渲染可能卡帧,考虑分页/虚拟滚动
   - **对策**: 异步 diff 计算 + 缓存 + 只渲染可见区域

2. **编辑器替换模式的渲染代价**:
   - `mountEditorReplacement` 会导致下面所有 widget 重排
   - ratatui 中 overlay 用 `Clear` 不会影响下层布局计算
   - **对策**: 将 overlay 区域定义为固定 `Rect`,下层 widget 感知 overlay 并腾出空间

3. **焦点管理的复杂性**:
   - kimi-code 有 3 级焦点:编辑器↔overlay↔全屏预览
   - ratatui 的 `Focus` 管理需要自定义或第三方库
   - **对策**: 用枚举统一管理焦点目标,键盘事件先分发到 `ActiveOverlay`,再 fallback 到常规焦点

4. **kitty 协议图片**:
   - kimi-code 支持 Kitty 内嵌图片(terminal inline images)
   - ratatui 无原生支持,需通过 raw escape sequence 输出
   - **对策**: 不推荐初期支持;可后期通过 `crossterm` 直接写 escape code

5. **状态同步竞争**:
   - approval/question 有队列和自动决议(session scope)
   - SDK 回调和 UI 回调的时序管理复杂
   - **对策**: 使用 `tokio::sync::oneshot` 通道,Controller 模式可复刻

6. **命令行参数验证与测试**:
   - kimi-code 的 `searchable-list.ts` 和 `paging.ts` 是共享实用层
   - **对策**: 先实现共享的 `SearchableList` 和 `PageView` 工具,所有 selector 复用它,减少重复代码

---

### 附录: 核心文件引用清单

| 文件 | 行数 | 重要性 |
|---|---|---|
| `tui/kimi-tui.ts` | ~2900 | ★★★★★ 主协调器,所有流程的总控 |
| `tui/tui-state.ts` | ~100 | ★★★★★ 全局状态定义 |
| `tui/commands/dispatch.ts` | ~300 | ★★★★★ 命令分发 |
| `tui/commands/registry.ts` | ~350 | ★★★★★ 命令注册表 |
| `tui/commands/resolve.ts` | ~130 | ★★★★★ 命令解析/路由 |
| `tui/commands/parse.ts` | ~20 | ★★★★ 命令字符串解析 |
| `tui/commands/types.ts` | ~30 | ★★★★ 命令类型定义 |
| `tui/reverse-rpc/types.ts` | ~150 | ★★★★★ 显示块类型系统 |
| `tui/reverse-rpc/base-controller.ts` | ~100 | ★★★★★ Controller 基类(队列+自动决议) |
| `tui/reverse-rpc/modal-coordinator.ts` | ~80 | ★★★★ 模态协调器(approval/question 互斥) |
| `tui/reverse-rpc/approval/handler.ts` | ~30 | ★★★★ 审批 handler |
| `tui/reverse-rpc/approval/adapter.ts` | - | ★★★ 审批数据适配 |
| `tui/reverse-rpc/approval/controller.ts` | ~30 | ★★★ 审批控制器 |
| `tui/components/dialogs/approval-panel.ts` | ~400 | ★★★★★ 审批面板(交互最复杂) |
| `tui/components/dialogs/question-dialog.ts` | ~790 | ★★★★★ 问题对话框(多 tab+review) |
| `tui/components/dialogs/model-selector.ts` | ~400 | ★★★★★ 模型选择器(基准实现) |
| `tui/components/dialogs/tabbed-model-selector.ts` | ~200 | ★★★★ tab 包装器 |
| `tui/components/dialogs/session-picker.ts` | ~400 | ★★★★ 会话选择器 |
| `tui/components/media/diff-preview.ts` | ~280 | ★★★★ diff 渲染引擎 |
| `tui/components/media/code-highlight.ts` | ~60 | ★★★ 代码高亮 |
| `tui/theme/colors.ts` | - | ★★★ 颜色 token 系统 |
| `.agents/skills/write-tui/DESIGN.md` | - | ★★★★★ TUI 设计规范(必读) |
| `.agents/skills/write-tui/SKILL.md` | - | ★★★★★ 架构方法论(必读) |
