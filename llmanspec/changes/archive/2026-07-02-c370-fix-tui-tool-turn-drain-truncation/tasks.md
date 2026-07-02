# c370-fix-tui-tool-turn-drain-truncation — Tasks

> 修复带 tool-call 的多轮 turn 被 TUI drain 在第一个 TurnEnd 提前截断的 bug。react loop 每个 ReAct 迭代发一个 TurnEnd（react.rs:453 在 tool 执行后、续轮前）；spawn_drain 收到第一个就 break，续轮事件全丢。改 drain 侧：stream None 才结束，turn 完成信号改由 forward task 发 Msg::XyDone。连带修复 cancel token 每 turn 重建（旧行为下全局 token 在首次 abort 后永久 cancelled，c370 改 drain 语义后会让后续 turn 全失效）。

## 0. 规划工件

- [x] proposal.md（draft→planned：补 design + tasks + spec delta）
- [x] design.md（方案 A：改 drain 侧 + Msg::XyDone + cancel 每 turn 重建）
- [x] specs/app-tui/spec.toon（delta：add tui52 multi-round-turn-not-truncated）
- [x] `llman sdd validate c370` 通过（tasks unchecked 为 strict 警告，实现后消失）

## 1. 回归测试（drain 行为）

- [x] `drain_runs_past_intermediate_turnend_to_stream_end`：构造 TextDelta→ToolEnd→中间TurnEnd→续轮TextDelta→最终TurnEnd，断言收到全部事件 + 续轮文字存活
- [x] `drain_stops_on_cancel`：cancelled token 让 drain 协作式停止（不跑完整流）
- [x] `fresh_token_drains_after_a_prior_cancel`：新 token 的 drain 不受旧 token cancelled 影响（cancel 复用回归）

## 2. 修复 drain 侧（app.rs）

- [x] `spawn_drain`：移除 `is_end` 检查 + 提前 break，只在 `cancel` 或 `None` 时退出
- [x] 删除 `turn_done`（语义已失效，由 Msg::XyDone 取代）

## 3. 流结束信号（mod.rs）

- [x] `Msg` 枚举加 `XyDone` 变体
- [x] forward task 退出前发 `Msg::XyDone`（drain drop xy_tx → recv None → 发 XyDone）
- [x] 主循环 `Msg::Xy` 分支：移除 `is_end`→`end_stream` 逻辑
- [x] 主循环新增 `Msg::XyDone` 分支：`app.end_stream()` + `draw_tail`

## 4. 连带修复：cancel 每 turn 重建（mod.rs）

- [x] `current_cancel: Option<Arc<CancellationToken>>` 取代全局 `cancel`
- [x] Submit 分支：中断旧 turn（take + cancel）+ 创建新 token 存入 current_cancel
- [x] Abort 分支：cancel 当前 turn 的 token（take + cancel）
- [x] `fresh_token_drains_after_a_prior_cancel` 回归测试覆盖

## 5. 校验

- [x] `cargo test --features tui --lib` 全绿（12 app 测试 + 70 渲染测试）
- [x] `cargo test --test bdd` 无回退（87 BDD 场景全过）
- [x] `just test` 563 全过（1 既存 leak 与本变更无关）
- [x] `cargo fmt --check` 干净
- [x] 同步本 tasks.md 勾选
