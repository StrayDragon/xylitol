# Preview：按代码 SSOT 映射，而不是按「组件库」幻想

> 纠正 `preview-looks-like.md` 里偏薄的 `fn(&mut HostSession)`。
> 难度在 **注入缝 + 正交平面**，不在再发明一套 Scene 类型。

## 1. 先承认：产品 TUI 不是可插拔组件树

代码事实（2026-08-14）：

| 层 | 是什么 | 是不是 `xylitol_tui::Component` |
|---|---|---|
| 包原子 | `Editor` / `SelectList` / `ChoicePrompt` / `Loader` / `Markdown` / `TreeSelector` | 是；产品 **拥有** 它们，不单独挂到 TUI 根 |
| 产品合成 | `render_scrollback` / `render_queue_strip` / `render_loaded_resources` | **否** — 纯函数，吃 `UiModel` + fold 状态 |
| 产品根 | `UiRoot`（~god）包在 `SharedUiRoot` | 产品树里 **几乎只有这一个** Component |
| 槽 | `EditorSlot` 互斥替换底栏 | 状态机，不是独立挂载的子树 |

`packages/xylitol-tui/AGENTS.md` 写得很清楚：通用组件在包里；**产品壳不进包**。
`src/app/tui/widgets/mod.rs` 也写了：这里 **不是第二套组件库**。

所以「所有 UI 组件可插拔可单独验证」如果理解成 Widgetbook 式拆树，会和现架构对打。
要对齐的是：**可观察表面** 能单独 **注入 + 看见 + 断言**，渲染仍走现在的 `UiRoot`。

不要先拆 `UiRoot` 再做 Preview。拆根是另一个高风险 change；Preview **骑在** 现有根上。

## 2. 该映射什么：表面，不是 Rust 类型

从代码 SSOT 映出 **用户能看见的面**。禁止「每个 struct 一条 Preview」（`ScrollbackPaintCache`、`FoldHitTable`、`SegmentClock` 不是面）。

### 2.1 Transcript 块（`UiEntry`）

`bridge/model.rs` 闭枚举，穷举本身就是 LSP 友好点：

| 变体 | 最小注入 | 至少一条 Preview 要盖住 |
|---|---|---|
| `User` / `Assistant` | live `TextDelta` 或 rebuild | flush 正文，不进 Used |
| `Thinking` | `ThinkingDelta` → flush；**另** rebuild 带 elapsed | live id ≠ 全局 `streaming_thinking`；封口 Thought 不被下一流改写 |
| `Tool` | `ToolExecutionStart/End` | Edit/Explore/Run/Used 角色（ActivityAtom） |
| `Ask` | tool name `ask` + Choice 槽 | 专用轨，不是 Tool 洗白 |
| `Diff` | display_diff | +/- 进簇头 |
| `Bash` | bang 流 / `UiModel` bash API | 交互块，非 agent tool |
| `Compaction` | `Compaction*` 事件 | 可省略簇头 |
| `Todo` | session todo 投影 | **投影不是 Used 调用** |
| `ScrollNotice` / `Error` | host `push_scroll_notice` | 尾随，≠ chrome toast |

流式尾巴 **不是** `UiEntry`：`UiModel.streaming_assistant` / `streaming_thinking`。live 窗必须用 **Xy 带**，手塞 entries 测不出。

### 2.2 正交平面：Activity fold

`ActivityFoldState` 住在 `UiRoot`，**不在** `UiModel`。
`apply_ui_model` 只拷贝 model；时钟 / L3 crush 走：

- live：`render_scrollback` 里 partition + 流式簇
- rebuild：`UiRoot::apply_activity_after_rebuild` ← `ingest_rebuild_clocks`
- turn-end：`apply_activity_after_turn_end`

因此「thought-only / Used N / 封口 vs live」是 **场景**，不是新组件。映射 3～N 条 fold 场景，不要映射一个 `ActivityFold` widget。

### 2.3 底槽（`EditorSlot`）

已有 `HostSession::mount_*` / `UiRoot::open_slot_for_test`：

| 槽 | 注入口 | 备注 |
|---|---|---|
| Editor | 默认 | 包原子；产品边框/补全在根上 |
| Choice | `mount_ask_choice` | 与 Ask 块成对 |
| Tree | `mount_session_tree` | 要假 `TreeNode`，经 XyDriver 的 travel 是另一条 |
| Models / Themes / Mcp / SessionResume / ImportConfirm | 对应 `mount_*` | 数据在 host，不在 `UiEntry` |
| Plate / Settings | stub | Evolving 即可，勿 Frozen |

### 2.4 Chrome（多数 **不在** `entries`）

| 面 | 状态在哪 | 注入 |
|---|---|---|
| 队列条 | `UiModel.pending_steer/follow_up` | Xy steer 事件或 `sync_queue` |
| 状态 Loader / 下轮预告 | `UiRoot.status_*` | busy + `set_status_next_turn_cue` |
| 壳层通告 | `UiRoot.chrome_toast` | `push_chrome_toast` |
| Copied | `copy_notice_until` | AO 鼠标路径 |
| 页脚 | cwd/model/thinking/tokens | `set_active_chrome` / footer token job |
| 已加载资源卡 | `loaded_resources` | snapshot，不是 transcript |
| TooSmall | `LayoutMode` | resize host |

这些必须 **单独列表面**。只注入 `Vec<UiEntry>` 永远看不见 toast / 下轮预告。

### 2.5 包原子

留在 `packages/xylitol-tui` 测试 + `agent_demo`。**不要**为 Editor 再做一条产品 Preview，除非测的是产品边框/slash/补全接线。

## 3. 三条注入缝（这才是架构难点）

| 缝 | 生产函数 | 能抓的 bug | 抓不到的 |
|---|---|---|---|
| **Live** | `HostSession::step(HostEvent::Xy)` → `handle_xy` → `apply_xy_event` → `sync_ui_root_from_model` | 流式 Thinking、inflight 工具、Choice 未封口 | resume 时钟、thinking id 与 JSONL 不一致 |
| **Resume** | `rebuild_scrollback_from_travel` + `apply_activity_after_rebuild` | 持久化 elapsed、travel 路径、封口 Thought | 流式帧、`streaming_*` |
| **Chrome** | `mount_*` / toast / footer / loaded-resources | 槽互斥、Esc 关槽、页脚 | 簇头词表 |

c1762 的教训正好跨缝：live id 作用域是 Live；persist elapsed 是 Resume。一条 `mount` 闭包若手搓 `UiEntry::Thinking`，**两条缝都测空**。

因此 Preview 的 mount **必须声明缝**，禁止默认「往 session 里塞 entries」。

### 建议的合法注入（类型，不是插件）

```rust
/// 唯一允许的产品 Preview 入口。穷举 = LSP 补全；禁止再加「随便 mutate UiRoot」。
pub enum PreviewInject {
    /// 与生产同一条：XyEvent 序列（可含 named checkpoints）。
    LiveXy(&'static [XyEvent]),
    /// 现成 live_tape 帧（activity-fold-live 已是这条）。
    LiveTape,
    /// JSONL / SessionEntry 种子 → rebuild + ingest clocks。
    Resume { scene_id: &'static str },
    /// 底槽 / toast / 页脚。可与 Live/Resume 组合。
    Chrome(ChromeOp),
}

pub enum ChromeOp {
    Toast(&'static str),
    NextTurnCue(&'static str),
    SlotChoice,      // 走 mount_ask_choice
    SlotTree,        // 走 mount_session_tree + 夹具节点
    SlotModels,
    // …
}
```

组合：`Resume { activity-fold-resume } + Chrome(SlotChoice)` 非法就别写；Ask 等待是 Live。
允许 `LiveXy + Chrome(NextTurnCue)`：transcript 与 chrome 本就分面。

`fn(&mut HostSession)` 只做 **逃逸口**（bang、slash、真 Driver），默认表不用它。

## 4. 「复用组件代码」具体指什么

复用，且已经存在：

1. **整壳**：`HostSession::new_product_ui(TestTerminal)` → 与 `/debug` 相同的 `SharedUiRoot`。
2. **transcript 纯函数**（仅语义 dump 的快路径）：`partition_segments` + `count_cluster` + `format_cluster_body`。这是 **断言材料**，不是第二套 paint。
3. **槽**：现有 `mount_*`，不要为 Preview 再写一套 SelectList 装配。

禁止：

- 为 Preview 再实现 `paint_fold_header(...)`
- 用 `agent_demo` 字符串当产品屏
- 用 playground HTML 当完成条件
- 把 `render_scrollback` 的输出当成「可以不经 `UiRoot::apply_ui_model`」的产品屏（cache / fold_hits / activity.row_spans 是根在 paint 时填的）

**推荐验证顺序（一条 Preview）**：

```text
PreviewInject  →  生产函数（step / rebuild / mount_*）
               →  UiRoot::render（或 session.render_now）
               →  去 ANSI 屏  +  从 UiRoot.activity + model.entries 抽语义 dump
               →  invariants(屏, dump)
```

dump **后置**于产品 paint，从 `ActivityFoldState` + `UiModel` 读，不要平行实现一套 segment。

## 5. 和 design / playground 的关系（仍中立）

| 物 | 继续当什么 |
|---|---|
| `design/*.md` MUST | 意图；可观察句逐步搬到对应 Preview invariants |
| playground HTML | 静图对照，**禁止**完成条件；不删也可以 |
| 代码 Preview | 动态 + 帧 + 交互的真值 |

「代码即 design、废文档」仍然 **不建议一步到位**：Helix/ratatui 也没靠 Storybook 废设计意图。先让 Preview 能看见，再让 Frozen 替代「散文 MUST 钉像素」。

## 6. 分阶段（难度如实）

这不是一个周末的类型提取。建议：

| 阶段 | 做什么 | 不做什么 | 谁 |
|---|---|---|---|
| **0 清单** | 本文件 §2 表变成仓库内 checklist（UiEntry × 缝 × 已有 `/debug`/测） | 不写 Preview 类型 | 派工 |
| **1 注入纪律** | 新夹具必须走 `PreviewInject` 三缝之一；禁止测里直接 `entries.push(Thinking)` 当产品测 | 不拆 UiRoot | 架构师审 + 派工改 harness 样板 |
| **2 dump** | 产品 paint 后打印 L3/L2/L1；c1762 三条场景用 dump 断言 | 不改词表 | 已派 `PROMPT-activity-fold-scene` |
| **3 表** | `DEBUG_SCENES` 长成 Preview 表（id + inject + stability + invariants）；inventory：每个 `UiEntry` 变体、每个非 stub `EditorSlot`、每个 chrome 槽 ≥1 条 | 不做侧栏 pantry | 架构师定类型后派工填表 |
| **4 看见** | `/debug <id>` 全部走同一表（含今天被排除的 `activity-fold-live`） |  | 派工 |
| **5 可选** | `just tui-preview` 侧栏；Frozen insta | 不引进 tui-pantry | 体验成熟后再做 |

ActivityAtom（穷尽 match、禁 `_ => {}`）是 **计数开闭**，与 Preview **正交**：Atom 防漏计；Preview 防漏看见。

## 7. 为何看起来「一点也不简单」

1. **状态分家**：transcript / activity 平面 / chrome / slot 四套，没有单一 `setState`。
2. **双投影**：live `apply_xy_event` vs resume `rebuild_scrollback_from_travel`，同一 `UiEntry` 形状、不同时钟与 id。
3. **God root**：交互（键、鼠标三角、Esc 关槽）只在 `UiRoot`+host 完整。孤立调用 `render_scrollback` 看不见 Choice、toast、折叠键。
4. **时间**：Loader / Thinking 流 / auto-degrade 依赖 clock；Frozen 不能对 spinner 拍黄金帧。
5. **性能**：多开占用的真问题在 AO 差分 + upper cache（`upper_gen`），不在 Preview 框架。夹具宽度/条数要像 `ao-perf-scroll` 那样显式，默认 80×24 测不出。

代价该花在 **钉注入缝 + dump + 清单**，不花在「组件可插拔运行时」。

## 8. 拍板用的一句话

> 产品 Preview = 声明 **注入缝** 的夹具，跑现有 `HostSession`/`UiRoot`，paint 后再读语义。
> 不是新组件系统，也不是把每个 Rust 类型变成 Story。
