# TUI 信息呈现与固定区词汇

> 固定讨论/文档用词，降低「system / message / trail」失真。**新文与 AGENTS 只准用本表**；代码标识符与本表同步。
> 跨端生命周期闭集见 [用户可见事件.md](./用户可见事件.md)。换模生效语义见 [运行时即时设置.md](./运行时即时设置.md)。

## 关键词汇（SSOT）

| 中文（讨论） | 英文（文档 / 标识符） | 含义 | 禁止混称 |
|---|---|---|---|
| **固定区** | fixed zone | 统称：主滚动区之外固定占位的框架区（状态条、页脚、通知条、待办栏、槽等 layout 壳） | 笼统 chrome / 壳层（弃用，见下表） |
| **待办栏** | todo bar | 下缘固定区常驻任务清单（队列条与通知条之间）；空表 0 行；**≠** 对话条目 / 通知条 / 状态条 | Plan 侧栏、`UiEntry::Todo` 当主清单 |
| **对话条目** | transcript entry | 主滚动区可持久内容（user / assistant / thinking / tool / …） | system 消息、LLM message（除非特指协议） |
| **滚动提示** | scrollback notice · **`UiEntry::ScrollNotice`** | 插入 **主滚动区** 的短 UI 提示 | system prompt、AgentMessage、system 消息；**勿**与通知条混称 |
| **通知条** | toast notice · **`ToastNotice`** · `push_toast_notice` | 通知类组件：底部输入区上方（status/spinner **之上**独立一行）的短暂固定通知；`{colors.warning}`；可见前缀 `Error: `；TTL 自动清除；**≠** 滚动提示 / 错误行 | 抢用 `ScrollNotice` / `UiEntry`；与 `UiEntry::Error` 混称；笼统 Notice 指滚动行 |
| **错误行** | error row | `UiEntry::Error` | 笼统 system |
| **状态条** | status | busy 时输入区上方短状态（idle = 0 行） | 塞进 scrollback |
| **页脚** | footer | 输入区下：生效中模型、用量 provenance 等 | 成功确认刷滚动提示 |
| **待生效** | pending | 状态：本轮生成使用开跑时绑定，`selected ≠ active` 仅在生成进行中短暂成立；footer 即时反映 selected（run 绑定语义，见 [运行时即时设置.md](./运行时即时设置.md)） | 挂账（弃用主词） |
| **队列条** | queue strip | steer / follow-up 条 | 对话正文 |
| **槽** | overlay / slot | 树、模型列表、resume… | 用滚动提示复述成功路径 |
| **尾插** | tail-append / append | append 到 entries **末**（跟底可见） | — |
| **顶插** | prepend | 写入 `entries[0]` 或等价前缀 | — |
| **Inline 交互** | **Inline** · `InteractionMode::Inline` | 主屏差分；**终端原生选区**取向（emulator-owned）；库 lab/demo | Mode A、inline-only 当唯一真名 |
| **ApplicationOwned 交互** | **ApplicationOwned** · `InteractionMode::ApplicationOwned` | 应用自管视口 + **应用内选区**；常经 alt-buffer + mouse capture；**产品缺省（ath30，AO 缺省）** | Mode B、alt-screen 当唯一真名（alt-buffer 只是 AO 常见载体） |
| **终端原生选区** | emulator-owned selection | 仿真器画选区/复制；应用不解释未修饰拖选 | 「开了 mouse 就有原生选区」 |
| **应用内选区** | application-owned selection | 应用收鼠标、自绘高亮、自复制（OSC52 等） | 把 mouse capture 说成「有选区」 |
| **timeout 预算注记** | `(timeout {N}s)` · tool header budget note | 工具行 header 在 `(Alt+E)` 前的 muted 预算声明；仅模型显式传 `timeout` 的 bash/grep/find 出现（c2435 tool-timeout-chrome），走默认不显示 | 倒计时（无）；把工具默认值逐行刷出 |

完整术语 ↔ 代码标识符对照（grabbed/ungrabbed、视口 vs scrollback）：`emulator-vs-app-selection-oneof.md` §1（`c2070`（双交互模式）research；2026-08-16 前的 change 已冷归档（freeze）进 `llmanspec/changes/archive/freezed_changes.7z.archived`）。

### 弃用（见旧文时对照本表改写）

| 旧词 | 改用 |
|---|---|
| 挂账（主词） | **待生效**（状态；仅生成进行中短暂成立） |
| **壳层通告**（主词） | **通知条**（c2550 定名；易误读为 OS shell，且本质是通知类组件） |
| **尾随** | **尾插**（与顶插成对；append 语义更直白） |
| chrome / 壳层（指固定框架区时） | **固定区**（代码标识符 `ChromeOp`/`push_chrome_toast` 等已随 c2550 改 `FixedZoneOp`/`push_toast_notice`） |
| Status trail / status trail / `status_trail` / 下轮预告 / next-turn cue / `Next turn: …` 文案 | 已随 attach run 绑定**退役**：产品端不渲染换模预告，勿再使用（见 [运行时即时设置.md](./运行时即时设置.md)） |
| 即将消息 | 弃用：产品无「即将消息」概念（run 绑定语义） |
| System 确认行 / system 消息（指 UI） / `UiEntry::System` | **滚动提示** / **`UiEntry::ScrollNotice`** |
| `push_system_note` | **`push_scroll_notice`** |
| 笼统 Notice 指滚动行 | **ScrollNotice**（瞬时硬拒反馈用 **通知条**，勿再堆滚动提示） |
| 把「尾插」写成 trail（无 append） | **尾插 / tail-append** |

换模预告的用户可见文案（`Next turn:` 等）已随 run 绑定语义从产品端退役。

## 信息分类 × 落点

| 类 | 典型内容 | 落点 | 持久？ | 放置默认 |
|---|---|---|---|---|
| **A 对话正文** | user / assistant / thinking / tool / diff / bash / compaction | 对话条目 | 是 | 按时间序 append |
| **B 导航瞬时** | `history @`、`forked →`、`switched →` | **滚动提示 · 尾插** | 随 scrollback；rebuild 可清 | **默认尾插**（跟底可见） |
| **C 操作结果 / 诊断** | slash 失败、复制、trust 报告 | 短：滚动提示尾插；成功换模/主题 → **不**刷 | 易堆墙 | **默认尾插** |
| **D 即时设置** | 换模 / thinking / 主题成功 | **页脚**（选中即时反映） | 否（态） | — |
| **E 运行态** | busy、abort、队列；busy 下硬拒反馈（如 Resume switch）；**待办栏**（常驻任务清单） | 状态条 / 队列条 / **通知条** / **待办栏** | 待办栏：有表驻留；通知条否 | 禁止冒充 A/B；硬拒反馈优先通知条，勿 ScrollNotice |
| **F 槽内确认** | 树 travel、选模 | 关槽 + 固定区 / B 类尾插 | 视 B/C | **默认尾插** |
| **G 减噪折叠** | 旧工具中间步；探索簇头计数后缀（`· N reads · M searches`，c2510/att35：调用次数 ≠ 文件计数，进行时 Exploring 同构） | 折叠摘要条目 | 是（形态变） | — |

**顶层原则（先于「禁顶插」口诀）**：绘制高效 + 跟底时用户仍能合理看到关键反馈。
- **默认尾插**滚动提示；顶插易导致视口外「假提示」并打散 per-index paint-cache → 通常更差。
- **不是绝对禁令**：若有明确产品理由且写清代价，可例外。
- 能进页脚 / 状态条 / 槽的，优先别做成滚动提示。

**原则一句话**：能反映在固定区（页脚 / 状态条 / **通知条** / **待办栏** / 槽）的不要做成滚动提示；必须进主区的瞬时信息 → **默认尾插**。

## 与代码的对应（可漂移，以代码为准）

| 概念 | 当前落点（摘要） |
|---|---|
| 滚动提示 | `UiEntry::ScrollNotice` · `HostSession::push_scroll_notice`；demo `Role::ScrollNotice` |
| 通知条 | host `push_toast_notice` · layout `toast_notice` 槽（status 上方；warning + `Error:`） |
| 待生效 | 生成进行中的 `selected ≠ active`（footer 即时反映 selected） |

## 维护

- 六个月后仍真？否则不要扩表。
- 新固定区能力先归类 A–G，再选落点；禁止静默发明第四套同义词。
- 端操作边界：[`src/app/tui/AGENTS.md`](../../src/app/tui/AGENTS.md)。
- **交互模型**：终端原生选区 vs 应用内选区（oneof）的跨端同源细则（库双入口、产品缺省、拖选 MUST、鼠标管道、`XYLITOL_TUI_MOUSE` 边界）→ [`../roadmaps/跨端同源.md`](../roadmaps/跨端同源.md)「支线与方向」表；词汇以本表 **Inline** / **ApplicationOwned** 词条为准。TUI 专属实现指针：段级 Activity 折叠见 `c1760`（activity 折叠，已归档）；Segment 鼠标命中见 `c2045`（剩余折叠目标）。
- **归档 change**（`llmanspec/changes/archive/`）可保留当时旧词作史实；**新文 / 活 docs / 活 specs（产品端）** 只准用本表。
- **designing**（仓库顶层 `designing/`）= 交互设计稿；`agent_demo` = 包演示，文案/固定区 **允许不同**。**禁止**把本表当成「必须改写 demo 字符串」的门禁。
