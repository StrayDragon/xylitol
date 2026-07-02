# c360 Future — 候选待办池

## Deferred Items

### 流式打字布局:tail 不应显示 pending 文字(→ c365)

**状态**:later(已确认要解决,开新变更 c365)

**问题**:c360 落地后真终端冒烟发现,当前 `draw_tail_frame` 把流式 pending 文字渲染在 thinking 指示器**上方**(顺序:流式文字 → thinking → 输入框)。用户期望的是 codex/pi 的 StreamController 双区域模式:**流式文字增量 commit 到 scrollback 边打字边生长,tail 区固定只留 thinking + 输入框**。

**为何不在 c360 做**:c360 是渲染基础设施变更(harness + widget 解禁 + RenderedLine seam)。完整的 mutable-last-line 需要 codex 那样的 StreamController(commit_tick + 稳定区/可变尾区分区 + table holdback,1800+ 行),属流式渲染架构,超出 c360 范围。

**触发信号**:用户真终端冒烟时确认了布局不符预期(流式 chunk 在 thinking 上方打字然后上行)。

**落地路径**:
- 后续 change id:`c365-add-streaming-mutable-tail`(或类似)
- 受影响 capability:`app-tui`(新增流式渲染 spec)
- 第一条动作:`llman-sdd-propose c365`,调研 codex `codex-rs/tui/src/streaming/`(controller.rs/commit_tick.rs/table_holdback.rs)的简化版
- 核心实现:scrollback 最后一行作为 mutable tail,TextDelta 增量更新该行而非 insert_before 新行;turn 结束才固定
- 关联代码:`src/app/tui/render.rs::draw_tail_frame`(移除 pending 渲染)+ `src/app/tui/app.rs::handle_xy_event`(TextDelta 改增量 commit)+ 可能需要 `InlineTerminal` 新增 mutable-last-line 能力

### Triggers to Reopen
- 真终端体验确认流式布局是高优先级 UX 问题(已确认)
