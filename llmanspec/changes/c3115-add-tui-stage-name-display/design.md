# Design — TUI 阶段名展示与首轮触发

## 目标

把 c3110 的阶段名接到产品 TUI：编辑区上边框稳定显示；fresh 会话首轮后自动一次；resume / fork 读取既有名；用户可 `/stage-name` 改名或刷新；失败只显示回退名，不打断主流程。

## 现状（代码事实）

- `UiModel` 在 `crates/xylitol-app-tui/src/bridge/model.rs`；已有 `entries` / `todo` / `mode_strip` / `streaming_*`；无阶段名字段。
- 编辑区上边框 chip 由 `crates/xylitol-app-tui/src/layout/root/footer_chips.rs:40` 生成；`append_border_chips`（`:61`）把 chip 画到第一行；`paint_editor_chips`（`layout/root/render.rs:10`）只在 editor slot 时替换第一行。
- resume rename 已存在：`crates/xylitol-app-tui/src/host/session_ops.rs:212`、`layout/root/mod.rs:1063`、`driver/remote.rs:1805`。
- `get_session_name` / `set_session_name` 命令已存在（`crates/xylitol-protocol/src/registry.rs:108-110`）；remote exec 已接（`driver/remote.rs:1805-1830`）。
- `SessionInfoEntry` 已在 entries 中；`session_display_name_from_entries`（`plugins/xylitol-official-plugin-session/src/manager/load.rs:393`）。
- `AgentEnd` 在 `crates/xylitol-app-tui/src/bridge/handlers/agent.rs:10`；host `handle_xy` 在 `host/mod.rs`。

## 数据流

```text
attach/resume
  → get_session_name (unary) 或 entries 里的 SessionInfoEntry.name
  → UiModel.stage_name
  → UiRoot.stage_name
  → 编辑区上边框 title

首个 AgentEnd（fresh、stage_name=None、未请求过）
  → pending.stage_name_request = true
  → effects drain: driver.generate_session_name(force=false)
  → HostEvent::StageName { name }
  → UiModel.stage_name = Some(name)
  → request_render

/stage-name <text>
  → PendingSlash::StageName(Set(text))
  → driver.set_session_name(text)
  → UiModel.stage_name = Some(text)

/stage-name refresh
  → PendingSlash::StageName(Refresh)
  → driver.generate_session_name(force=true)
  → 更新或保留既有名
```

## 触发与幂等

- 每个 session id 记一个 `stage_name_requested: bool`（TUI 本地，不持久化）。
- 只在 `AgentEnd` 且 `phase` 从 Busy → Idle 的第一个 turn 触发；若 `stage_name` 已有则不触发；若该轮以 Aborted / Error 结束，则跳过自动生成（用户可 `/stage-name refresh`）。
- host 侧 `generate_session_name(force=false)` 也检查既有名，双保险；重复调用返回既有名。
- 触发是异步的：不进入 `step_paint_only` 同步路径；失败只 warn，不产生通知条/错误块。
- 生成中不显示假 Busy / 假 Reloading；标题为空时边框保持原样。

## 展示

- 位置：编辑区上边框右端（`╮` 左侧），右对齐，不居中；例如 `╭──── 新功能A ───╮`。
- 颜色：muted；不抢 accent；激活态不加反色。
- 宽度：先扣除左右圆角与右侧留白；剩余宽度给标题；标题超宽用 `truncate_to_width` + `…`；不得换行、不得增高。
- 无阶段名：不留空槽，边框保持纯 `─`。
- 标题与边框同色？DESIGN 允许上边框承载视图动作；阶段名用 muted 以区别于 chip 的 thinking 档位色。最终以 c3130 的视觉预览为准。

## Slash 语义

- `/stage-name`：当前名 → 滚动提示「阶段名：X」；未生成 → 「阶段名未生成」。
- `/stage-name <text>`：trim；空 → 提示；成功 → 滚动提示「阶段名已更新」；失败 → 通知条/滚动提示按既有 slash 错误风格。
- `/stage-name refresh`：显式刷新；成功 → 更新；失败 → 保留既有名 + 滚动提示。
- 与 resume rename 共用 `set_session_name`；不新增第二套写路径。
- 解析：`crates/xylitol-app-tui/src/commands.rs` 的 `PendingSlash`；`layout/root/slot_input.rs` 的输入策略；`dispatch.rs` 的 `DispatchOutcome`。

## 协议 / driver

- c3110 新增 `Command::GenerateSessionName { force: bool }` 与 `generate_session_name` unary；TUI remote 增加 `exec_generate_session_name`。
- in-process driver 已有 `XyDriver`；新增方法由 c3110 定义。
- `get_session_name` / `set_session_name` 复用现有 exec；不新增命令。

## 实施任务

1. `UiModel` / `UiRoot` 增加 `stage_name`；bridge 处理 `HostEvent::StageName`；`sync_ui_root_from_model` 同步。
2. attach / resume 读取：优先 entries 的 `SessionInfoEntry.name`；否则 `get_session_name` unary；设置标题。
3. 首轮触发：`AgentEnd` 后置 pending；effects drain 调 `generate_session_name(false)`；每个 session 一次。
4. slash：`/stage-name` 查看 / 改名 / refresh；解析 + dispatch + 错误文案；复用 `set_session_name`。
5. 标题绘制：`append_border_chips` / `paint_editor_chips` 加 title 段；宽度预算 + `…` 截断；无标题不占位。
6. fork：切换后读父名；不自动生成；subagent 不显示。
7. 测试：harness 首轮触发一次、resume 读取、slash 改名/refresh、标题截断、无标题、fork 继承；snapshot 80/120 列。
8. specs：`app-tui-fixed-zone` 阶段名展示；`app-tui-commands` `/stage-name`；必要时 `app-tui-input`。

## 验证

- harness：首轮 AgentEnd 只调用一次 `generate_session_name`；已有名不调用；resume 不调用。
- snapshot：80 / 120 / 160 列标题截断与不增高；无标题时边框与现状一致。
- slash：查看 / 改名 / refresh 的成功与失败；错误只走滚动提示/通知条，不伪造 Busy。
- 真机 tmux：首轮后标题出现；改名；refresh；resume 恢复；fork 继承。

## 风险 / 待确认

- 标题落位与 c3130 的 chip / 字形 / 圆角改版共享同一行；c3115 先按「chips 左侧」落位，c3130 再重排。若 c3130 先做，c3115 的 snapshot 需重基。
- 首轮触发时机：`AgentEnd` 之后 TUI 可能已经空闲；异步 unary 返回后只更新标题，不改变 Busy/Idle。
- 生成延迟数秒：标题可能晚于首轮出现；可先显示回退名（host 返回最终名，MVP 不先回退）。
- fork 父名复制由 c3110 负责；若父名为空，TUI 不生成（避免 fork 自动重命名）。
- 阶段名与 session resume rename 共用存储；用户改名即钉住，refresh 才覆盖。
