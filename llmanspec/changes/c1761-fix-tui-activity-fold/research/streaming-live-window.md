# 流式 live window（c1761 深挖）

对照 Cursor Agent 窗截图（Thought / Explored / Editing / Planning next moves）。
视觉 MUST：[`src/app/tui/design/activity-fold.md`](../../../src/app/tui/design/activity-fold.md)；静图槽 `activity-fold`。
代码事实：xylitol 已有 `streaming_thinking` / `streaming_assistant` 双缓冲；`flush_streaming` 在 ToolStart 等处提交；busy status 短词是 `Working` / `Running {name}`（**状态条**，不是本条 transcript 尾行）。产品尚无 `Planning next moves`。

## 切段（MUST）

**簇边界 = 已落定的 LLM 助手正文**（`UiEntry::Assistant` 或非空 `streaming_assistant` 开始可展示）。

| 事件 | 切簇？ |
|---|---|
| ThinkingDelta / 落定 Thinking | **否**（thinking 是活动，不是正文） |
| Tool / Bash / Ask / Diff / Todo / Compaction | **否**（同一打开簇内累计） |
| 助手正文 **第一个非空白字符** | **是**：封上一个打开簇 |
| 仅空白 / 尚未可展示的正文 | **否**；尾行用 `Planning next moves` |

这替换 design 初稿里「按工具种类连续切簇」作为 **主切刀**。种类（Explored / Edited / Ran / Thought）只影响 **同一打开簇的摘要措辞**，不单独切段。

## Live window（流式当前 turn）

从输入向上数：

| 位置 | 角色 | 流式中 |
|---|---|---|
| **-1 尾行** | 动态折叠块：当前 agent 操作 | 每步可换文案；可含 Thinking 流、Ask 等待 |
| **-2 打开簇头** | `Editing …` / `Explored …`（进行时） | **计数与措辞随 ToolEnd 更新** |
| **-3 及更旧** | 已封簇（`Edited` / `Explored` / `Thought 20s`）或已画助手正文 | **冻结**：不重算、不 invalidate |

「最新一条消息还不足以展示」= 还没有可画的助手正文（空缓冲 / 仅空白）→ **-1 必须是 `Planning next moves`**，禁止画空 Assistant 气泡。

### -1 尾行状态（推荐，互斥）

| 条件 | 文案（英文，对齐既有 L2 模板） |
|---|---|
| busy 且无可展示助手正文，且无进行中 thinking / tool / Ask | `Planning next moves`（**无**折叠三角） |
| `streaming_thinking` 非空 | `Thinking`（展开可跟 thinking 正文；时长有可靠戳才写 `Thought Ns`） |
| Ask `Waiting`（堵塞） | `Asking questions`；**Ask 块必须可交互**（Choice 槽），禁止折进已折叠簇里看不见 |
| 工具进行中 | 短操作行，如 `Editing tests.rs` / `Reading a.rs` / `Running ls` |
| 助手正文已可展示 | **无**独立尾行；正文本身就是最新消息 |

Ask 与 thinking 都是「最新动态折叠块」的合法内容，不是切段刀。

### -2 打开簇

- 进行时：`Editing 7 files, explored 6 files, 2 searches, ran 5 commands +301 -8`（类目在 **ToolStart** 加；`+/-` 仅 ToolEnd 且有可靠 diff）
- 被助手正文封口后：同一模板改过去式 `Edited …`，此后该行进入 -3 冻结集
- 新工具结束后只改这一行的数字，不碰 -3

## 与 `stream_collapse` 的关系（修订）

先前默认「流式当前 turn 整段套 `Worked for` 信封」**与截图矛盾**（流式时可见 Thought / Explored / Editing / Planning next moves 多层）。

修订推荐：

| 范围 | 行为 |
|---|---|
| **流式当前 turn** | 始终 live window（-1/-2 动，-3 冻）；**不**整段收成 Worked for |
| **已结束 / resume 超窗 turn** | `stream_collapse: envelope` → `Worked for` 信封；`clusters` → 只留簇头 |

`envelope` 的少刷收益在流式中改为：**-3 冻结 + paint cache**，而不是把当前 turn 藏进一行信封。

## 与状态条解耦

`Planning next moves` 落在 **对话条目 / 减噪折叠（G）**，不是状态条短词。
既有 `Working` / `Running {name}` 状态条 **本票默认保留**（不把 Planning 搬进 spinner 行）。禁止把尾行叫成 status。

## 打开簇 = 队列（低级自动收拢）

对照新发现：`Planning next moves` **可点**；收回走 **-2 `Edited…` 上的折叠三角**；三角显隐随「已累积、可展开的子块」变化。低级操作自动流进上一聚合块，像队列：一部分在飞，一部分已收纳。

```text
-3  已封簇 / 已画正文          冻结
-2  Editing 7 files…  ▸/▾     打开簇头 = 队列已收纳部分（sealed）
-1  Planning next moves        队列在飞部分（inflight）；可点；默认无三角
```

| 动作 | 效果 |
|---|---|
| ToolStart | 类目 +1；inflight = 该工具短行（`Editing tests.rs`） |
| ToolEnd | inflight **并入** sealed；只改 -2 摘要字符串；若簇未展开则不画子块 |
| 无 inflight | -1 = `Planning next moves` |
| 点 -1 | **展开 -2 簇**（揭开已后台算好的 sealed 子块 + 当前 inflight） |
| 点 -2 三角 | 同一簇 toggle；折叠后回到头行 + -1 尾行 |
| 助手正文首非空白 | 整簇封口 → 过去式，进入 -3 |

三角何时出现在 -2：`sealed` 非空（有可展开的累积）。仅 inflight、尚无 sealed 时 -2 可暂不画三角（无可收纳）。

多级树（低级自动聚合，不是任意行区间）：

```text
turn 信封 Worked for
  └ 簇 Edited/Explored     ← 工具结束后自动收进这里
       └ 块 read/grep/ask  ← 更低级；默认不画，点开簇才揭开
```

旧 turn 自动把簇收进信封；流式当前 turn 只自动把块收进打开簇。

## 点击揭开 = 展示已算好的，不现算

「点击后实时把后台运算直接展示」的实现约束：

| 层 | 何时算 | 点击展开时 |
|---|---|---|
| 簇摘要字符串、计数、+/- | ToolStart / ToolEnd 增量写进簇状态 | 只读缓存字符串，O(1) 画头行 |
| sealed 子块 | 只存 `UiEntry` 下标，不拷正文 | 按下列表现有 `render` 路径；命中 per-entry paint cache |
| 中间助手正文（信封折叠藏、展开揭） | 条目早已在 `entries` 里 | 展开信封时取消 skip，不重跑 Markdown（ath25） |
| -3 | 指纹不变 | 不进本次 invalidate |

禁止：点击瞬间全量 `partition_segments`、重解析历史 Assistant、重扫整个 scrollback。

Invalidate 集合（流式）：`{-1 尾行, -2 头行}`；簇展开时再加 sealed 子块的 entry 下标。其它行走 paint cache。

点 -1 的命中：整行可点（与 att22 三角列-only 的 **例外**，同类于 att30 hint 带）；**不**在 Planning 行画三角。真正的 ▸/▾ 只在 -2。
