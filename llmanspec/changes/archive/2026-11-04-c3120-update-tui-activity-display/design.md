# Design — TUI 活动折叠等级预设与子项可见性

## 目标

把「折叠密度」做成一等预设 + 逐子项覆盖：默认 summary（流式 thinking 一行，消除跳动）；quiet 全部折叠；recent 最近一轮展开；full 全展开。运行时与配置都可切换；折叠仍为端侧纯呈现。

## 现状（代码事实）

- `ActivityFoldSettings` 只有 `enabled` / `keep_recent_turns` / `auto_on_rebuild` / `auto_on_turn_end` / `collapse_floor`（`crates/xylitol-app-tui/src/activity_fold/settings.rs:4`）。
- `SegmentLevel` 只有 L0 / L2 / L3（`activity_fold/segment.rs`）；L0 无信封、簇默认收起；L2 信封头 + 簇头；L3 信封全折。
- `paints_envelope_header` 只在 `collapse_floor==L3` 为真（`settings.rs:25`）；`collapse_floor=L2` 时信封头全消失。
- `enabled=false` 返回 L0（`state.rs:57`），但簇仍默认收起（`cluster_is_expanded` `state.rs:188`），与 r1356「关闭则全细账」字面有出入。
- 流式 thinking 默认展开：`ScrollbackFold::thinking_effective` 对 `STREAMING_THINK_ID` 返回 true（`widgets/scrollback/mod.rs:77`）；`paint_folded_streaming_thought`（`live.rs:109`）画簇头 + 正文。
- spec r1372 / r1364 锁定「流式 thinking 正文默认展开」；r1356 锁定自动收纳默认；r1739 锁定配置字段。
- 现有动作：`app.thinking.toggle` / `app.tools.expand` / `app.tools.blocks` / `app.activity.expandNearest` / `app.activity.collapseNearest`（`keybindings.rs:33-49`），默认无和弦。
- opencode 的可见性开关（tool details / thinking）是独立持久化布尔，不是预设；本设计取「预设 + 逐项覆盖」的混合。

## 显示等级语义

| 等级 | 已结束轮 | 当前轮 thinking | tool header | tool output | diff | 最近簇 |
|---|---|---|---|---|---|---|
| quiet | 全部 L3 | 一行 `Thinking` | 簇头一行 | hidden | hidden | 收起 |
| summary | L3 + 近 2 轮 | 一行 `Thinking` | 簇头一行 | 5 行 preview | 12 行 preview | 收起 |
| recent | L3 + 近 1 轮 | 全文 | 簇头一行 | 5 行 preview | 12 行 preview | 轮末展开 |
| full | 不自动折 | 全文 | 工具块头可见 | 全文 | 全文 | 全部展开 |

- 流式 thinking `header`：画 `▸ Thinking · <最新非空行，按剩余宽度截断>`；正文不画；高度 1 行；结束后 `▸ Thought Ns`。`full`：现有正文流；`hidden`：不画 thinking 块，状态条保留 Thinking。
- 工具块 `summary`：只画簇头（如 `▸ Ran 1 command`），不画工具块头；`full`：工具块头 + 块体；`hidden`：不画簇头（只留信封/正文）。
- output / diff `preview`：沿用现有 5 / 12 行 viewport；`full`：展开；`hidden`：不画。
- `full` 档需要显式展开所有 `cluster_open` 与 segment L0；不能只靠 `enabled=false`（当前不展开簇）。

## 配置

```yaml
tui:
  activity_fold:
    display_level: summary        # quiet | summary | recent | full
    visible_items:
      thinking: header            # hidden | header | full
      tool_header: summary        # hidden | summary | full
      tool_output: preview        # hidden | preview | full
      diff: preview               # hidden | preview | full
    # 既有字段仍可用，显式值覆盖预设
    enabled: true
    keep_recent_turns: 2
    stream_collapse: envelope
    auto_on_rebuild: true
    auto_on_turn_end: true
```

- 优先级：`visible_items` > 预设；显式 `keep_recent_turns` / `stream_collapse` > 预设；`enabled=false` 仍表示不套信封（full 档另走展开路径）。
- 非法值：未知等级 / 未知模式 → 配置加载失败（与 r1739 的 stream_collapse 一致）。
- 默认：summary；thinking=header；tool_header=summary；tool_output=preview；diff=preview。

## 运行时开关

- 动作 `app.activity.displayLevel`：循环 quiet → summary → recent → full；默认无和弦；进 r3032 目录。
- `/fold`：
  - `/fold` 或 `/fold status`：滚动提示当前等级 + 覆盖项。
  - `/fold quiet|summary|recent|full`：切换等级；清空该等级下被预设覆盖的逐项覆盖？设计：切等级只换预设基线，显式覆盖保留；`/fold reset` 清覆盖。
  - `/fold reset`：清空逐项覆盖，回到预设。
  - `/fold thinking header|full|hidden` 等：写单项覆盖。
- chip：c3130 在编辑区**下边框右端**画 `[▤ 摘要]`（用户已确认文案），点击循环；窄档隐藏。
- 切换等级是事件边界一次批量写 `levels` / `cluster_open` / 可见性默认；不得每帧改折叠态（r1364）。

## 状态与渲染改造

- `ActivityFoldSettings` 增加 `display_level` 与 `visible_items`；`from_preset` 生成默认。
- `ScrollbackFold` 增加 `thinking_mode` / `tool_header_mode` / `tool_output_mode` / `diff_mode`；保留既有 bool + per-id overrides 作为点击后的局部覆盖。
- `thinking_effective(STREAMING_THINK_ID)` 改为按 `thinking_mode`：`full` → true；`header` → false 但画一行；`hidden` → 不画。
- `paint_folded_streaming_thought` 增加 header 模式：取 `streaming_thinking` 最后非空行，`truncate_to_width`，加 `▸ Thinking · ` 前缀；不画正文。
- `paint_cluster_header_row` 按 `tool_header_mode`：summary 只画簇头；full 画簇头 + 工具块；hidden 不画簇头。
- `paint_tool_block` 的 output/diff 按 mode：hidden 跳过；preview 用现有 viewport；full 展开。
- `full` 档：新增 `expand_all_clusters` / `set_all_segments_l0`；轮末不 auto degrade。
- `quiet` 档：`keep_recent_turns=0` + 流式 thinking header + output/diff hidden；已结束轮 L3。
- `recent` 档：轮末 `expand_live_cluster`（事件边界一次）；旧轮 L3。
- 修复 `paints_envelope_header`：L2 也应画信封头；或删除 L2 语义只保留 L3。设计取「L2 画信封头 + 簇头」以保持现有 stream_collapse=clusters 语义。
- 修复 `enabled=false`：明确它只去信封，不展开簇；full 档不依赖它。若要保持 r1356「全细账」字面，full 档用 `enabled=false + expand_all_clusters`。

## Spec 变更点

- r1356：增加 display_level / visible_items 与 full/quiet 语义；修正 `enabled=false` 描述。
- r1372：流式 thinking 默认展开 → 按等级；summary/quiet 一行 header；recent/full 全文；更新场景。
- r1364：流式 thinking 正文默认可见 → 按等级；工具块默认折叠 → 按 tool_header；更新场景。
- r1739：新增 display_level / visible_items 字段与默认；非法值失败。
- r3032：新增 `app.activity.displayLevel` 默认空和弦。
- 新增规则：显示等级预设、逐子项可见性、`/fold` 语义。

## 实施任务

1. config：`DisplayLevel` / `VisibleItems` 类型、默认、校验、schema；单测。
2. settings/state：`from_preset`、`expand_all_clusters`、`set_all_segments_l0`、`paints_envelope_header` 修复。
3. scrollback fold：模式字段 + `thinking_effective` + 局部覆盖优先级。
4. paint/live：streaming thinking header、cluster header summary/hidden、output/diff mode。
5. 动作/命令：`app.activity.displayLevel`、`/fold` 解析与 dispatch；键位目录。
6. 事件边界：切等级批量应用；recent 轮末展开；full 不 auto degrade。
7. 测试：harness 四档 snapshot；流式 thinking header 高度稳定；逐项覆盖；`/fold`；config 非法值。
8. specs：上述规则与场景；BDD binding。

## 验证

- harness：summary 流式 thinking 从首帧到 flush 始终 1 行；quiet 只留信封；recent 最近簇展开；full 全部展开。
- snapshot：四档在同一 entries 下的帧；无 `Worked for` 误套；无 `Planning next moves`。
- 配置：默认 summary；非法等级/模式失败；显式覆盖优先。
- 真机 tmux：切换四档、`/fold`、点击簇头/块头；观察高度稳定。

## 研究

- opencode 可见性开关摘录：[`research/opencode-visibility.md`](./research/opencode-visibility.md)。
- tmux 帧观测（thinking 跳动）：[`../c3125-add-tui-transcript-folds/research/tmux-long-input-and-tool-args.md`](../c3125-add-tui-transcript-folds/research/tmux-long-input-and-tool-args.md)。

## 风险 / 待确认

- r1372/r1364 是现行硬条款；本 change 是行为合约变更，需在 propose 时明确 WARNING 与场景更新。
- `full` 档是否等于 `enabled=false` 需定；设计取「full = 不自动折 + 全部展开」，与 `enabled=false` 解耦。
- `quiet` 是否隐藏簇头（tool_header=hidden）还是保留一行摘要；设计取保留信封 + 隐藏簇头，具体以预览确认。
- chip 落位为下边框右端，由 c3130 实现；本 change 只提供动作与命令。
- 逐项覆盖的持久化：端侧会话级内存；配置文件是默认值；不写 Host。
