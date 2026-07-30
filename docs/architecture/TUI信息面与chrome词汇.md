# TUI 信息面与 chrome 词汇

> 固定讨论/文档用词，降低「system / message / trail」失真。**新文与 AGENTS 只准用本表**；代码标识符与本表同步。
> 跨面生命周期闭集见 [用户可见事件.md](./用户可见事件.md)。NextTurn 行为见 [运行时即时设置.md](./运行时即时设置.md)。

## 关键词汇（SSOT）

| 中文（讨论） | 英文（文档 / 标识符） | 含义 | 禁止混称 |
|---|---|---|---|
| **对话条目** | transcript entry | 主滚动区可持久内容（user / assistant / thinking / tool / …） | system 消息、LLM message（除非特指协议） |
| **滚动提示** | scrollback notice · **`UiEntry::ScrollNotice`** | 插入 **主滚动区** 的短 UI 提示 | system prompt、AgentMessage、system 消息；**勿**与角落弹层混称 |
| **（预留）壳层通告** | notice / toast（未来） | 非 scrollback 的短暂通告（如角区弹层）；**≠** 滚动提示 | 抢用 `ScrollNotice` 或笼统 Notice 指滚动行 |
| **错误行** | error row | `UiEntry::Error` | 笼统 system |
| **状态条** | status | busy 时输入区上方短状态（idle = 0 行） | 塞进 scrollback |
| **页脚** | footer | 输入区下：生效中模型、用量 provenance 等 | 成功确认刷滚动提示 |
| **待生效** | pending | 状态：`selected ≠ active`，下一 turn 边界才起用 | 挂账（弃用主词） |
| **下轮预告** | **next-turn cue** | busy status **行右侧** dim 文案（用户可见常为 `Next turn: …`） | Status trail、trail、即将消息、pending message |
| **队列条** | queue strip | steer / follow-up 条 | 下轮预告、对话正文 |
| **槽** | overlay / slot | 树、模型列表、resume… | 用滚动提示复述成功路径 |
| **尾随** | trail-append / trailing | append 到 entries **末**（跟底可见） | 与 next-turn **cue** 不同根概念 |
| **顶插** | prepend | 写入 `entries[0]` 或等价前缀 | — |

### 弃用（见旧文时对照本表改写）

| 旧词 | 改用 |
|---|---|
| 挂账（主词） | **待生效**（状态）或 **下轮预告**（UI） |
| Status trail / status trail / `status_trail` | **下轮预告** / **next-turn cue** / `status_next_turn_cue` |
| 即将消息 | **下轮预告**（不是 message） |
| System 确认行 / system 消息（指 UI） / `UiEntry::System` | **滚动提示** / **`UiEntry::ScrollNotice`** |
| `push_system_note` | **`push_scroll_notice`** |
| 笼统 Notice 指滚动行 | **ScrollNotice**（预留 Notice/toast 给角区等壳层通告） |
| 把「尾随」写成 trail（无 append） | **尾随 / trail-append** |

用户可见文案 `Next turn:` / `Next turn thinking:` **可保持**；改的是概念名与标识符，不是强迫改屏上字符串。

## 信息分类 × 落点

| 类 | 典型内容 | 落点 | 持久？ | 放置默认 |
|---|---|---|---|---|
| **A 对话正文** | user / assistant / thinking / tool / diff / bash / compaction | 对话条目 | 是 | 按时间序 append |
| **B 导航瞬时** | `history @`、`forked →`、`switched →` | **滚动提示 · 尾随** | 随 scrollback；rebuild 可清 | **默认尾随**（跟底可见） |
| **C 操作结果 / 诊断** | slash 失败、复制、trust 报告 | 短：滚动提示尾随；成功换模/主题 → **不**刷 | 易堆墙 | **默认尾随** |
| **D 即时设置** | 换模 / thinking / 主题成功 | **页脚 + 下轮预告**（待生效时） | 否（态） | — |
| **E 运行态** | busy、abort、队列 | 状态条 / 队列条 | 否 | 禁止冒充 A/B |
| **F 槽内确认** | 树 travel、选模 | 关槽 + chrome / B 类尾随 | 视 B/C | **默认尾随** |
| **G 减噪折叠** | 旧工具中间步（候补） | 折叠摘要条目 | 是（形态变） | — |

**顶层原则（先于「禁顶插」口诀）**：绘制高效 + 跟底时用户仍能合理看到关键反馈。
- **默认尾随**滚动提示；顶插易导致视口外「假提示」并打散 per-index paint-cache → 通常更差。
- **不是绝对禁令**：若有明确产品理由且写清代价，可例外。
- 能进页脚 / 状态条 / 下轮预告 / 槽的，优先别做成滚动提示。

**原则一句话**：能反映在 chrome 的不要做成滚动提示；必须进主区的瞬时信息 → **默认尾随**。

## 与代码的对应（可漂移，以代码为准）

| 概念 | 当前落点（摘要） |
|---|---|
| 下轮预告 | `status_next_turn_cue` · `status_next_turn_cue_text` · playground `.status-next-turn-cue` |
| 滚动提示 | `UiEntry::ScrollNotice` · `HostSession::push_scroll_notice`；demo `Role::ScrollNotice` |
| 壳层通告（未来） | 未实现；勿占用 `ScrollNotice` |
| 待生效 | selected ≠ active（模型 / thinking） |

## 维护

- 六个月后仍真？否则不要扩表。
- 新 chrome 能力先归类 A–G，再选落点；禁止静默发明第四套同义词。
- 面操作边界：[`src/app/tui/AGENTS.md`](../../src/app/tui/AGENTS.md)。
