# TUI 对标调研总索引

> 四个仓库的 TUI 实现横向对比，服务于 xylitol 的 TUI 基础设施打磨决策。
> 生成日期：2026-07-02 ｜ 分支：feat/tui-inline-repl
>
> **注意（2026-07-10）**：下文「xylitol 家底 / ratatui 手写渲染」描述的是**已删除的旧 TUI**。
> 现行产品面为 `packages/xylitol-tui` + `src/app/tui` host 空壳（冻结中）。
> 现行 core 就绪度与命名债见：**[2026-07-10-src-core-tui-readiness.md](./2026-07-10-src-core-tui-readiness.md)**。

## 现行审计（2026-07-10）

| 报告 | 内容 |
|---|---|
| [src-core-tui-readiness](./2026-07-10-src-core-tui-readiness.md) | `src/` 相对产品 TUI：事件/队列/Driver/trust 就绪度 + 命名探索（只读） |

## 历史对标报告（2026-07-02）

| 仓库 | 技术栈 | 报告 | 一句话定位 |
|---|---|---|---|
| **codex** | Rust + ratatui(umbrella, fork) | [codex.md](./codex.md) | 同栈，工业级参考；177K 行，含 36 帧黄金快照 |
| **ratatui** | ratatui-core/widgets/crossterm 源码 | [ratatui.md](./ratatui.md) | 积木库本身；TestBackend 是测试基石 |
| **pi** | Ink(TS，自研非 React) | [pi.md](./pi.md) | 30 个组件的交互范式；diff/选择器最全 |
| **kimi-code** | Ink(TS，pi-tui) | [kimi-code.md](./kimi-code.md) | overlay 体系最复杂；30+ dialogs + 命令分发 |

---

## 一、关键决策回顾：xylitol 当时 TUI 家底（历史快照，已过时）

```
src/app/tui/   1480 行，8 个文件
├── mod.rs      236  run() 主循环 + 事件分发
├── render.rs   443  ★ 手写 cell-by-cell 渲染 + CJK 换行
├── app.rs      262  TuiApp 状态
├── terminal.rs 133  InlineTerminal 封装
├── input.rs    146  键盘读取
├── commands.rs 106  ★ slash 命令（半残：/model 构造了 Command 却丢弃）
├── theme.rs     71  配色
└── init.rs      83  终端初始化 + panic hook
```

**c341 的设计约束**（`llmanspec/changes/archive/2026-07-01-c341-*/design.md:27`）：
> **不加** `ratatui-widgets`（umbrella 才有；我们弃用所有内置 widget）。

**现实**（render.rs:14 处手写 Buffer/cell 操作）：
- 文本换行手写 `wrap_to_width`（CJK 感知）
- 边框/输入框/状态栏全部手写 cell 循环
- **ratatui 报告指出**：`Paragraph::new(text).wrap(Wrap)` 已正确处理 CJK/emoji/零宽——手写版是重复造轮子且易出 off-by-one

---

## 二、Widget 横向对比总表

按 coding agent TUI 必备能力分组建表。"xylitol 现状" 列反映当前实现方式。

### 2.1 消息渲染

| 能力 | codex | pi | kimi-code | xylitol 现状 | 建议 |
|---|---|---|---|---|---|
| 用户消息回显 | `UserHistoryCell` | `UserMessageComponent` | `UserMessageComponent` | ✅ `render::user_message_line` | 维持，接 `Paragraph` |
| assistant 文本流式 | `AgentMessageCell` + markdown | `AssistantMessageComponent` | `AssistantMessageComponent` | ✅ tail 区流式 | 流式分区算法可借鉴 codex |
| thinking 折叠/展开 | `ReasoningCell` | `ThinkingSelector` | `ThinkingComponent` | ❌ 无 | **后建** |
| 消息分部件枚举 | `HistoryCell` enum(20+ 变体) | thinking/text/toolCall 顺序 | 22 个消息组件 | ❌ 无统一抽象 | **建议先建 enum 骨架** |

### 2.2 输入

| 能力 | codex | pi | kimi-code | xylitol 现状 | 建议 |
|---|---|---|---|---|---|
| 多行文本输入 | `ChatComposer`(11203 行!) | `CustomEditor` | `CustomEditor` | ✅ 单行 input.rs | 多行先延后 |
| @ mention 弹窗 | ✅ `@`/`$` | ❌ | `FileMentionProvider` | ❌ | 后建 |
| paste burst 检测 | ✅ | ? | ? | ❌ | 后建 |
| slash 命令补全 | 内嵌 | 内嵌 | `FileMentionProvider` | ❌ | 后建 |

### 2.3 对话框 / overlay（xylitol 当前最大缺口）

| 能力 | codex | pi | kimi-code | xylitol 现状 | 建议 |
|---|---|---|---|---|---|
| overlay 架构 | `BottomPaneView` 栈 trait | `ctx.ui.custom({overlay:true})` | `mountEditorReplacement` 替换模式 | ❌ 完全无 | **核心：先建 overlay 抽象** |
| 工具审批对话框 | `ApprovalOverlay`(2409 行) | `ToolExecutionComponent` 状态色 | `ApprovalPanelComponent`(400 行) | ❌ | **必备**（agent 要执行工具） |
| 模型选择器 | `ListSelectionView` | `ModelSelectorComponent` | `TabbedModelSelectorComponent` | ❌ | **必备**（`/model` 现在半残） |
| 通用列表选择器 | `ListSelectionView`(2810 行) | 多个 Selector | 共享 `SearchableList` 工具类 | ❌ | **建议抽公共组件** |
| 全屏查看器 | `PagerOverlay` | - | `ApprovalPreviewViewer`(快照替换) | ❌ | 后建 |
| 问题采集(多问题表单) | `RequestUserInputOverlay` | - | `QuestionDialogComponent`(790 行) | ❌ | 后建 |

### 2.4 chrome（常驻外壳）

| 能力 | codex | pi | kimi-code | xylitol 现状 | 建议 |
|---|---|---|---|---|---|
| 状态栏(footer) | `Footer`(2082 行) | `FooterComponent` | `FooterComponent` | ✅ `StatusLine` | 接 `Paragraph` |
| spinner/加载 | `StatusIndicatorWidget`+`Shimmer` | `BorderedLoader` | `MoonLoader` | ⚠️ 文字 "..." | 低优先 |
| todo 面板 | `PlanCell` | - | `TodoPanelComponent` | ❌ | 后建 |

### 2.5 富内容渲染

| 能力 | codex | pi | kimi-code | xylitol 现状 | 建议 |
|---|---|---|---|---|---|
| diff 渲染 | `DiffRender`(2534 行) | `renderDiff`(词级) | `DiffPreview`(聚类+省略) | ❌ | **后建，但有现成 diff_review 死码可参考** |
| 代码高亮 | `MarkdownRender`(syntect) | cli-highlight | `CodeHighlight`(cli-highlight) | ❌ | 后建 |
| markdown | `MarkdownStreamCollector` | `Markdown` 组件 | `AssistantMessageComponent` | ❌ | 后建 |

---

## 三、测试 Harness 横向对比（用户的重点关切）

| 维度 | codex | ratatui(库) | pi | kimi-code |
|---|---|---|---|---|
| **渲染断言** | `TestBackend` + `insta` 快照 | `TestBackend` + `assert_eq!(buf, Buffer::with_lines)` | `render(width)` 返字符串 + `stripAnsi` + `toContain` | `render(80).join` + `strip` + `toMatch` |
| **多帧/序列** | VT100Backend + AppEvent 通道 | `terminal.draw()` 多次 + `assert_buffer_lines` | 直接多次 render | handleInput 模拟按键 |
| **fake 数据** | AppEvent/AppCommand 通道 + LazyLock 模型预设 | - | `createFakeTui` + `createBaseToolDefinition` | `makePending()` 工厂 |
| **错误路径** | 构造 `TurnError::CodexError` 协议结构体 | - | 同正常路径，喂错误 result | 同 |
| **按键模拟** | ❌ 不模拟按键，直接调 App 方法 + EventBroker 注入 | ❌ TestBackend 不支持事件 | 直接 `handleInput('\x1b')` | `handleInput(DOWN)` |
| **依赖终端?** | 否(纯内存) | 否(纯内存) | 否 | 否 |

**三家共同模式（强共识）**：
1. **渲染到内存 buffer → 断言内容**，绝不依赖真终端
2. **fake provider/event 喂确定数据**（正常格式 + 错误格式）
3. **状态驱动**：改变 state → render → 断言输出，不测中间调用

**xylitol 当前测试现状**：30 个 TUI 测试，但报告暗示多为逻辑断言。**缺 TestBackend 渲染断言层**——这正是用户要的"减少写死单测、构筑可验收的 harness"。

---

## 三-bis、关键架构事实：三家全是 inline 模式（非 alt-screen）

> **纠正一个常见误解**：inline vs alt-screen 是两种根本不同的终端接管方式。三家 coding agent TUI **全部是 inline**。

| 项目 | 模式 | `?1049h` alt-screen | 滚动机制 | 退出后历史 |
|---|---|---|---|---|
| **codex** | inline (`Viewport::Inline`) | ❌ 无 | 终端原生 scrollback | ✅ 保留 |
| **pi** | inline（自研差分渲染） | ❌ 无 | 终端原生 scrollback | ✅ 保留 |
| **kimi-code** | inline（pi-tui 差分） | ❌ 无 | 终端原生 scrollback | ✅ 保留 |

**证据**（全仓 grep `1049/smcup/rmcup/enterAltScreen` 零命中）：
- codex：`Viewport::Inline` + `insert_before`，spec tui1 明文"MUST NOT enter alternate screen"
- pi：`packages/tui/src/terminal.ts:134-167` `start()` 只开 raw mode + bracketed paste，无 `?1049h`；退出 `stop()` 也无 `leaveAlt`
- kimi-code：pi-tui 引擎同 pi，无 alt-screen；连 `clearScreen()` 定义了都从不调用

**机制区别**：
- **inline**（三家所选）：已完成行通过 `\r\n` 自然滚进终端原生 scrollback → 用户用**原生滚轮**滚动 → 退出后历史仍在终端。代价：必须自己管理逐帧差分、viewport 游标。
- **alt-screen**（vim/htop 式，三家都**没选**）：进 `?1049h` 接管整屏 → TUI 内部 buffer 存历史 → 退出后清空。

**对 xylitol 的含义**：当前 `Viewport::Inline` 选型**与三家一致，是正确方向**。目标形态不是"切到 alt-screen 做浮层"，而是**在 inline 模式内增加布局复杂度**（transcript 区 + 底部对话框/输入区，如 kimi 的"替换底部输入区"模式）。harness 仍基于 `TestBackend + Viewport::Inline`（ratatui 官方范例 `ratatui/tests/terminal.rs:66` 支持）。

---

## 四、四个仓库的可视化界面对比

### codex — 工具审批（`ApprovalOverlay` + `ListSelectionView`）
```
  Would you like to run the following command?

  Thread: Robie [explorer]

  $ echo hi

› 1. Yes, proceed (y)
  2. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel or o to open thread
```
> 特点：选项是 ListSelectionView 渲染的列表，统一选择抽象。

### pi — 流式对话 + 工具执行 + footer
```
┌─────────────────────────────────────────────────────┐
│  Let me check the codebase for this...               │
│  _The model is reasoning about the approach..._      │ ← thinking(italic)
│ ┌─────────────────────────────────────────────────┐ │
│ │ $ rg -n "important" src/                        │ │ ← bash 执行框
│ │ src/main.rs:42:fn important_function() {        │ │
│ │ ⠋ Running... (esc to cancel)                    │ │
│ └─────────────────────────────────────────────────┘ │
│ ~/project (main)                                     │ ← footer
│ ↑1.2k ↓0.5k CH25.0% $0.012 45%/128k auto            │
└─────────────────────────────────────────────────────┘
```
> 特点：工具执行有边框 + 流式输出 + spinner；footer 信息密度高。

### kimi-code — overlay 替换编辑器区域（不叠加，是替换）
```
│  Kimi K2 is thinking...                              │ ← transcript 区保留
│  ❯  I'll update the README with the new API docs.   │
│  ──────────────────────────────────────────────────  │
│    ▶ Run this command?                               │ ← approval 替换了
│      curl -X POST https://api.example.com/deploy     │   输入框区域
│    1. Approve once                                   │
│    2. Approve for session                            │
│  ❯ 3. Reject                                         │
│    4. Reject with feedback                           │
│    ↑/↓ select · 1/2/3/4 choose · ↵ confirm           │
│  ──────────────────────────────────────────────────  │
│  Kimi K2 · plan · manual · 1.2k/200k  [Queue: 2]    │ ← footer
```
> 特点：**overlay 不是浮层，是替换 editorContainer**（底部输入框位置）。transcript 不动。

### xylitol 当前 — 极简 inline REPL
```
xylitol — type a prompt and press Enter. /exit to quit.
❯ 你好
  ← 用户输入回显(刚加的)
  你好！我是 xylitol...            ← tail 区流式(8 行)


❯ _                              ← 输入行(底锚)
```
> 现状：**无 footer 状态栏、无对话框、无工具执行展示、无 diff**。只有单行输入 + 流式 tail。

---

## 五、决策建议：下一步 widget 待建清单

### 优先级 P0（harness + 核心抽象，先有验收再写功能）

| # | 任务 | 依据 |
|---|---|---|
| 1 | **建 TestBackend 渲染测试 harness** | ratatui 报告 §5.2 已给三层方案；codex 用同款 `insta + TestBackend`；四家共识 |
| 2 | **重新评估 c341 "弃用 widgets" 决策** | ratatui 报告 §5.3 指出 `Paragraph` 已正确处理 CJK，手写 wrap 是重复造轮子。**这是用户说的"写死单测有害"的根源**——手写渲染绑死了实现细节 |
| 3 | **建 `HistoryCell`/消息部件 enum 骨架** | codex 20+ 变体是成熟范式；pi 的 thinking/text/toolCall 顺序；xylitol 现在无统一抽象 |

### 优先级 P1（agent 可用的最小闭环）

| # | 任务 | 依据 |
|---|---|---|
| 4 | **overlay 抽象 + 焦点状态机** | 三家都有；kimi 的 `mountEditorReplacement` 替换模式最简单，可先实现 |
| 5 | **通用 `SearchableList`/列表选择器** | kimi 全部 selector 复用一个工具类；codex 的 `ListSelectionView`；pi 的 `ModelSelector` |
| 6 | **模型选择器对话框** | `/model` 当前半残（c335 也卡这）；三家都有标准实现 |

### 优先级 P2（交互完整性）

| # | 任务 | 依据 |
|---|---|---|
| 7 | **工具执行审批对话框** | agent 要执行工具必备；codex/pi/kimi 都有 |
| 8 | **diff 渲染组件** | kimi 聚类算法 + pi 词级；xylitol 有 diff_review 死码可参考(c350) |
| 9 | **footer 状态栏** | 三家都有，信息密度参考 pi |

### 暂缓（用户明确：先打磨基础，不堆功能）

- thinking 折叠、@mention、多行编辑、代码高亮、markdown、todo 面板、问题采集表单
- 以及 c330/c335/c345/c350 四个功能提案（都是功能，延后）

---

## 六、可视化：xylitol TUI 当前 vs 目标形态

> 全程 **inline 模式**（不进 alt-screen），与 codex/pi/kimi 一致。已完成内容滚入终端原生 scrollback，用户用原生滚轮滚动，退出后历史保留。

```
【现状：极简 inline REPL】              【目标：inline 多区块（P1-P2，参考 kimi）】
                                          （仍在 inline 模式，非浮层）
┌──────────────────────────┐           ┌──────────────────────────┐
│ greeting                 │           │ ...transcript（原生        │ ← 已完成行
│ ❯ 用户输入回显            │           │   scrollback，可滚轮上滚）  │   滚进原生
│   assistant 流式 tail     │           │ ❯ 用户输入回显             │   scrollback
│                          │           │   assistant 流式 tail      │
│                          │           │   ──────────────────────  │ ← footer
│ ❯ _                      │           │   model · 1.2k/200k       │
└──────────────────────────┘           └──────────────────────────┘
                                                ↓ 触发 /model 或审批时
                                       ┌──────────────────────────┐
                                       │ ...transcript 保留不动... │
                                       │   ────────────────────── │
                                       │   Select a model          │ ← 替换底部
                                       │   ❯ gpt-5.1   openai      │   输入区
                                       │     kimi-k2  kimi         │  （kimi 模式：
                                       │   ────────────────────── │   非浮层，
                                       │   model · 1.2k/200k       │   非悬浮窗）
                                       └──────────────────────────┘
```

**关键**：目标形态的"对话框/选择器"不是悬浮在 transcript 上的浮层（那是 alt-screen 思路），而是**替换底部输入区**（kimi 的 `mountEditorReplacement` 模式）——transcript 区保持不动，仍由原生 scrollback 承载。这也是为什么 inline 模式不需要 `Clear` widget 做层叠。

## 七、待用户决策的开放问题

1. **是否重新评估 c341 的 "弃用 widgets" 决策？**
   ratatui 报告强烈建议引入 `ratatui-widgets`（至少 `Paragraph`/`Block`/`List`/`Clear`），可省 ~50 行手写 CJK 换行 + 边框代码。这是"减少写死单测"的关键——用库的 widget，测试断言的是行为而非实现。

2. **overlay 走哪种模式？**
   - (a) kimi 的"替换编辑器区域"（最简单，单层）
   - (b) codex 的 `BottomPaneView` 栈（多层嵌套，复杂）
   - (c) ratatui `Clear` + 居中浮层（传统，需手写 z-order）

3. **harness 落在哪？**
   - (a) `src/app/tui/` 内 `#[cfg(test)]` 模块（组件级）
   - (b) `tests/tui_render.rs` 集成测试（端到端）
   - (c) 两者结合（推荐：组件级断言 + fake provider e2e）

4. **是否需要建独立的 `xylitol-tui` crate？**
   codex 把 TUI 单独成 crate。xylitol 目前在 `src/app/tui/`。打磨基础时是否值得提升为独立 crate？
