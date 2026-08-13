# Design: ActivityFold 嵌套 + 配置

对照：[`c1760`](../archive/2026-08-12-c1760-add-tui-activity-fold/design.md) 平面 L0/L2/L3。本票把「一段一行摘要」换成 **信封套簇**，L1 块级仍正交。

**视觉 MUST**：[`src/app/tui/design/activity-fold.md`](../../../src/app/tui/design/activity-fold.md)；静图槽 `activity-fold`（`just open-design-playground`）。TUI 高度变化 = 一次 paint，**不**做滑入动画。

## 产品树

展示 **扁平同列**：信封头、簇头、块头与未折叠细账同一列；层级只存在于每个折叠块自己的状态里，**MUST NOT** 用前导缩进表达嵌套。

```text
[User]                                      始终外显
▸/▾ Worked for {duration}                   信封（展开后头行仍在）
▸/▾ Edited foo.rs, explored …               簇（可多条；Todo/Compaction 算活动）
▸ read / grep / edit / thinking             块 L1
▸/▾ Ran 3 commands
[该 turn 最后一段 Assistant]                 信封折叠时唯一保留的正文
ScrollNotice / Error                        永不进信封
```

**折叠信封（B）**：User + `Worked for` + **最后一段** Assistant。中间助手正文不画。

**信封展开 / 流式 live window**：**已钉**仍画出夹在簇之间的中间正文（对齐截图）；仅折叠信封时执行 B（中间正文不画）。

## 可折叠空间（嵌套独立开关）

不是整段只有一档 L0/L2/L3，而是可创建块的嵌套空间：各级 **独立** toggle；「重叠」= 祖先与子孙同时展开。禁止任意行区间交叉折叠。

新活动（ToolStart / Thinking / Compaction / Todo）并入打开簇；助手正文首个非空白字符封口。

## 级与旧平面对照

| 旧 | 新 | 用户看到 |
|---|---|---|
| L3 | 信封折叠 | User + `Worked for` + **最后** Assistant |
| L2 | 信封展开、簇折叠 | User + ▾ `Worked for` + 簇头 + **中间正文仍画** + 最后 Assistant |
| L0 | 某簇展开 | User + ▾ `Worked for` + 该簇细账（L1）+ 其它簇头 + 最后 Assistant |

展开任一级 **MUST** 留下该级头行 + `▾`（att19）。这是 c1760 paint 只在 `is_collapsed()` 画摘要行的根因修复。

## 簇切分（可测启发式）

**主刀：LLM 助手正文。** 一段打开簇 = 两次可展示助手正文之间的全部活动（Tool / Thinking / Diff / Ask / **Todo / Compaction**）。用户 bang 命令块不进信封。Thinking **不**切段。种类只影响该簇摘要措辞（Explored / Edited / Ran / Thought），不单独切段。

封口：助手正文开始可展示 → 打开簇改过去式并冻结，进入 live window 的 -3。

- Planning next moves / Asking questions / 打开簇进行时：见 [`research/streaming-live-window.md`](./research/streaming-live-window.md)（已钉）。

空簇 MUST NOT 生成。切分 live ≡ resume（同一 entries 序 → 同一簇边界）。簇 id 建议 `seg-{user_idx}:c{cluster_ord}`。

## 配置

落到 `AppConfig.tui.activity_fold`（`src/infra/config`），host 装入 `ActivityFoldSettings`。缺省 = 产品默认。非法 `stream_collapse` MUST 使配置加载失败。

| YAML 键 | 默认 | 含义 |
|---|---|---|
| `enabled` | `true` | 关则全细账（L1 仍可用）；resume 也不套信封 |
| `keep_recent_turns` | `2` | 仅约束 `auto_on_turn_end`：最近 K 个已结束 turn 信封默认展开；更旧：信封默认折叠 |
| `stream_collapse` | `envelope` | 见下 |
| `auto_on_rebuild` | `true` | travel/resume/fork 后对**全部**已结束 turn 套折叠信封（不按 keep 留近窗） |
| `auto_on_turn_end` | `true` | 回合结束后对超窗 turn 套折叠信封 |

`stream_collapse`（**只作用于已结束 turn**；流式当前 turn 见 live window）：

| 值 | 已结束 turn | 流式当前 turn |
|---|---|---|
| `envelope` | `Worked for` 信封（rebuild=全部已结束轮；turn-end=超窗） | live window：-1/-2 更新，-3 冻结；**不**整段套信封 |
| `clusters` | 只留簇头 | 同上 live window |

流式少刷 = 冻结 -3 的 paint cache，不是把当前 turn 收成一行 Worked for。

旧键 `auto_l3_distant` **不**再暴露 YAML；行为由信封折叠 = Worked for 覆盖。

示例：

```yaml
tui:
  activity_fold:
    enabled: true
    keep_recent_turns: 2
    stream_collapse: envelope   # envelope | clusters
    auto_on_rebuild: true
    auto_on_turn_end: true
```

## 交互

### 鼠标（沿 att31 / ath33）

- 点 **信封** 三角列：只 toggle 该 turn 信封（展开 → 见簇头；折叠 → 只见 Worked for）。
- 点 **簇** 三角列：只 toggle 该簇（信封必须已展开）。
- 信封折叠时簇/块三角 MUST NOT 登记。
- 簇折叠时块三角 MUST NOT 登记（att25）。
- 禁止用 `expandNearest` 代替定点。

`FoldTarget`：保留 `Segment(id)` 为信封；新增簇目标（产品级：稳定簇 id）。同一 `FoldHitTable`，禁止第二管道。

### 键盘 C1

`expandNearest` / `collapseNearest` 改为嵌套栈（仍一对和弦，无第三默认键）：

- expand：距输入最近的 **折叠信封** 先展开；若已展开则展开最近折叠簇。
- collapse：距输入最近的 **展开簇** 先折；若无则折最近展开信封。近窗从未进入的细账仍不是 collapse 目标。
- 无目标静默。

旁注：信封/簇头仍用满段和弦，**不**写 `(Alt+E)`。

## 动态刷新与性能

细则：[`research/streaming-live-window.md`](./research/streaming-live-window.md)。「后台」= host 同步增量，不另开线程。

| 事件 | MUST | MUST NOT |
|---|---|---|
| 无可展示助手正文 | -1 = `Planning next moves` | 空 Assistant 气泡 |
| ThinkingDelta | 刷新 -1（Thinking / 展开体） | 重算 -2/-3 |
| TextDelta（助手正文） | 刷新正在流的正文；封口后打开簇进 -3 | 全量 `partition` + 全历史 MD |
| ToolExecutionEnd / Bash 完成 | 只更新 -2 打开簇计数 | 动 -3 |
| Ask Waiting | -1 = `Asking questions`；Ask 块可交互 | 折进已折叠簇 |
| TurnEnd / rebuild | rebuild：全部已结束轮按 `stream_collapse` 套信封；turn-end：超窗套信封或簇头 | 假时长 |

ath25：信封/簇 toggle MUST NOT 触发全部历史 Assistant Markdown 重解析。

## 时钟

沿 c1760：rebuild 从 session 树两端戳；live 用 turn 边界。缺任一端 → 信封仍可画 `Worked for` **无时长**（att24）。禁止用墙钟「现在」冒充结束戳。

## 测试 seam

沿用产品 harness（`HostSession` + 合成 Mouse / 键）与 `activity_fold` 模块测；配置用 `AppConfig` YAML 单测。不新开脱离 `.feature` 的 CLI 边界。可执行场景保持 `feature: false` unit（与 att19–att32 一致），除非落地时改口。
