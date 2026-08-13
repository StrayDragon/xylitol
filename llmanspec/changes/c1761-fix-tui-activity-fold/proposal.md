---
depends_on:
- c1760-add-tui-activity-fold
- c2045-add-tui-fold-target-remaining
branch: sdd/c1761-fix-tui-activity-fold
base_sha: f2f2a7fed255aab800547eb1c12f85883da0e1be
checkpointed: false
---

# ActivityFold 嵌套收纳 + YAML 配置

> **一句话**：嵌套折叠（旧 turn：Worked for 信封 → 簇 → 块）+ 流式 live window（切段=LLM 正文；-1 当前操作 / -2 打开簇会动；-3 冻结）；YAML `tui.activity_fold`。

> 前置已归档：[`c1760`](../archive/2026-08-12-c1760-add-tui-activity-fold/)（段状态机）· [`c2045`](../archive/2026-08-12-c2045-add-tui-fold-target-remaining/)（Segment 鼠标）。本票 **修**「L0 丢掉摘要头 / 无再折入口」，**加** 嵌套与配置面。Compaction 头行三角同行不在本票（quick 另交）。

## Why

c1760 把一整段中间操作收成 **一行** 摘要；点开后摘要行消失，只剩独立 Tool/Thinking 块——没有 `▾` 再折入口（与 att19「展开可收起」及 Cursor Agent 窗不一致）。配置只在代码默认，resume 无法按用户意图显示 `Worked for`。流式时若仍画全部细账，刷新成本高。需要对齐 Cursor：**信封套簇套块**、头行始终可点、配置控制自动收纳深度、折叠块内容可随工具结束增量刷新。

## What Changes

1. **嵌套折叠**（取代「一段一行」平面）：
   - **信封**：`Worked for …`（有可靠双端戳才写时长；禁止伪造）。
   - **簇**：`Edited … / Explored … / Ran …`（可多条/每 turn）。
   - **块**：既有 L1 Tool/Diff/Ask/Thinking。
   - 展开信封/簇后 **头行仍在**，标记 `▾`，可再折。
2. **可见地板（折叠信封）**：User + Worked for + **仅该 turn 最后一段 Assistant**；中间助手正文视为活动，随工具/thinking/Todo/Compaction 一起被收。ScrollNotice / Error **仍不进**。
3. **可折叠空间**：多级树；**低级操作自动收进上一聚合块**（块→簇→信封）。各级可独立展开。点 `Planning next moves` = 展开 -2 簇（揭开已算好的 sealed）；收回用 -2 三角。不是任意行区间交叉折叠。
4. **YAML** `tui.activity_fold`（映射到既有 `ActivityFoldSettings` + 新枚举；缺省 = 推荐默认）。
5. **`stream_collapse`**（语义名，不用 A/B）：
   - `envelope`（默认）：**已结束 / resume 超窗** turn 收成 `Worked for` 信封。
   - `clusters`：超窗 turn 只留簇头，不套信封。
   - **流式当前 turn 不套信封**（见下条 live window）。
6. **流式 live window**（当前 turn）：
   - 切段刀 = **已可展示的 LLM 助手正文**；Thinking / Tool / Ask **不**切段。
   - 打开簇 = **队列**：sealed 进 `-2 Edited…`；inflight 为 `-1`。
   - `-1`：`Planning next moves` **可点、无三角**（点 = 展开 -2，揭开已算好的 sealed）。Thinking 流、Ask 堵塞、进行中工具必须正确表达。
   - `-2`：打开簇头；**三角只画在这里**（sealed 非空时）；ToolStart 更新类目。
   - `-3` 及更旧：冻结。
7. **resume/rebuild**：`enabled` 且 `auto_on_rebuild` 时，超 `keep_recent_turns` 的 **已结束** turn 显示折叠信封（`Worked for`）。
8. **键盘 / 鼠标**：`expandNearest` / `collapseNearest` 嵌套一级步进；点 -1 与点 -2 三角同簇。
9. **性能**：后台增量写摘要；点击只揭缓存；只 invalidate -1/-2；ath25。

## Capabilities

| capability | 角色 |
|---|---|
| `app-tui-transcript` | att23–att28 / att31 嵌套、持久头行、stream_collapse 产品行为 |
| `runtime-config` | `tui.activity_fold` YAML 键与非法值失败 |
| `app-tui-host`（MAY） | 配置装入 host；键位动作若需改路由 |

## 已拍板

| 项 | 决定 |
|---|---|
| 拆票 | 本票 = ActivityFold；Compact 三角同行 = **quick**（非本 change） |
| 模型 | Cursor 嵌套（信封 → 簇 → 块），不是只补 L0 头行 |
| 可见地板 | 信封折叠：User + Worked for + **最后** Assistant |
| `-1` 三角 | Planning **无三角但可点**（= 展开 -2）；真正 ▸/▾ 在 Edited…（sealed 非空） |
| 流式当前 turn | **live window**（-1/-2 动，-3 冻）；不套 Worked for |
| `stream_collapse` | 只约束 **已结束/超窗** turn：默认 `envelope` |
| 切段刀 | LLM **助手正文**；thinking 不切 |
| 信封折叠可见 | **B**：User + Worked for + **仅最后一段** Assistant；中间正文随活动一起被收 |
| Todo / Compaction | **进信封**（Todo 现为普通工具；Compaction 对齐 Cursor；ScrollNotice/Error 仍不进） |
| 可展示 | 助手正文 **第一个非空白字符** 即封口并开始画 |
| `-1` 三角 | `Planning next moves` **无**三角；Thinking/Ask 可折；`Editing tests.rs` 不是独立层 |
| `-2` 计数 | 文件/搜索/命令 **ToolStart** 加；`+/-` 仍 ToolEnd + 可靠 diff |
| 折叠空间 | 可创建的嵌套块；各级 **独立** 展开/折叠（信封 ∩ 簇 ∩ 块），不是互斥单档 |
| 不足展示 | `-1` = `Planning next moves`（禁止空 Assistant 气泡） |
| Ask 堵塞 | `-1` = `Asking questions`；Ask 块必须可交互，不得折没 |
| 配置名 | 语义名 `envelope` / `clusters`，不用 A/B |
| L3 时长 | 沿 att24：缺戳省略时长，禁止伪造 |
| 近窗 | `keep_recent_turns` 默认 2：近窗信封默认展开、簇可自动折；更旧信封默认收 |
| `auto_l3_distant` | **退役**：折叠信封即 Worked for；远/近由 keep_recent + rebuild/turn-end 控制 |

## 与邻接边界

| 对方 | 本票 | 对方保留 |
|---|---|---|
| c1760 | 升嵌套；修展开无三角 | 段时钟、C1 动作 id、L1 正交 |
| c2040 / c2045 | 扩展 Segment 命中为信封+簇；不重做 L1 四类 | Tool/Diff/Ask/Thinking / Compaction / Viewport |
| Compact quick | 不改 Compaction 头行 | 三角与 `[compaction] Compacted` 同行 |

## Out of scope

- session / LLM compaction 算法
- 假 `Worked for` / 假 `+/-`
- 把折叠做成任意行号区间的几何重叠（只做嵌套独立开关）
- 本票升格 Todo 视觉 / 状态栏一体化（仍当普通工具进信封）
- Web 实现（动作语义可回写同源板一句）
- Compaction 块布局（quick）
- c1505 viewport slice / c1535 wrap

## Open Questions

| # | 题 | 决议 |
|---|---|---|
| 1 | 拆票 | **已钉**：c1761 propose + Compact quick |
| 2 | 嵌套 vs 只补头行 | **已钉**：嵌套 |
| 3 | 可见地板 | **已钉**：User + 头 + 最终 Assistant |
| 4 | 流式粒度 | **已钉**：当前 turn = live window；`envelope` 只用于已结束/超窗 |
| 5 | 簇切分 | **已钉**：主刀 = 助手正文；种类只影响摘要措辞 |
| 6 | `Planning next moves` | **已钉**：英文；transcript -1 尾行；不进状态条 |
| 7 | 状态条 Working | **已钉**：保留 `Working` / `Running {name}`，与尾行并存 |
| 8 | Ask 堵塞 | **已钉**：-1 = `Asking questions`；Ask 块可交互，不得折没 |
| 9 | 打开簇时态 | **已钉**：进行中 `Editing`；正文封口后 `Edited` |
| 10 | 信封×中间正文 | **已钉 B**：折叠信封只留最后 Assistant |
| 11 | Todo/Compaction | **已钉**：进信封；ScrollNotice/Error 不进 |
| 12 | 可展示阈值 | **已钉**：首个非空白字符 |
| 13 | `-1` 三角 | **已钉**：Planning 无三角 |
| 14 | 计数时机 | **已钉**：类目 ToolStart；+/- ToolEnd |
| 15 | 任意折叠空间 | **已钉**：多级树 + 低级自动收拢；点 -1 揭开 -2；三角在 Edited… |
| 16 | 展开揭内容 | **已钉**：后台增量算好；点击只揭缓存，不现算 |
| 17 | 信封展开/live 中间正文 | **已钉**：折叠信封才藏；live/展开仍画 |
| 18 | TUI 展开动画 | **已钉**：一次 paint 揭开；playground 用离散芯片，不要求滑入 |

## Ethics

- risk_level: low–medium
- prohibited_actions: 伪造时长或 +/-；折叠冒充 session compaction；TextDelta 全量重画历史 Assistant；默认分支改 live specs
- required_evidence: 展开后头行 `▾` 可再折；resume 超窗 `Worked for`；流式 -2 随 ToolEnd 变、-3 不变；无正文时见 `Planning next moves`；Ask Waiting 可交互；YAML 非法枚举加载失败
- escalation_policy: 若簇切分要改 JSONL / 进 LLM 上下文 → 停下来确认

## Start readiness

| 项 | 值 |
|---|---|
| **ready_for_start** | **yes**（视觉已锁；L1 块级折叠走既有 expandable，playground 可不接线） |
| Specs | 绑定分支后改 att23–att28、att31 + 流式 live window req + `runtime-config`；`feature: false` unit |
| 禁止 | 默认分支改 `llmanspec/specs/**`；把 Compact 头行改动算进本 change |

流式细则：[`research/streaming-live-window.md`](./research/streaming-live-window.md)。
