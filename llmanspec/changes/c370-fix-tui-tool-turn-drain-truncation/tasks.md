# c370-fix-tui-tool-turn-drain-truncation — Tasks

> 修复带 tool-call 的多轮 turn 被 TUI drain 在第一个 TurnEnd 提前截断的 bug。react loop 每个 ReAct 迭代发一个 TurnEnd（react.rs:453 在 tool 执行后、续轮前）；spawn_drain 收到第一个就 break，续轮事件全丢。改 drain 侧：stream None 才结束，turn 完成信号改由 forward task 发 Msg::XyDone。

## 0. 规划工件

- [x] proposal.md（draft→planned：补 design + tasks + spec delta）
- [x] design.md（方案 A：改 drain 侧，加 Msg::XyDone）
- [x] specs/app-tui/spec.toon（delta：add tui52 multi-round-turn-not-truncated）
- [x] `llman sdd validate c370 --strict` 通过

## 1. 复现测试（先写，红）

- [ ] 用 `FakeModel` 构造多轮 tool-call 序列：`TextDelta("Let me check")` → `FunctionCall(bash)` → tool 执行 → `TextDelta("The answer is 42")` → `Done`
- [ ] 断言续轮 `TextDelta("The answer is 42")` 到达 TUI 并 commit 到 scrollback（修复前应失败）
- [ ] 断言 `end_stream` 恰好调用一次（流结束时），非两次

## 2. 修复 drain 侧（app.rs）

- [ ] `spawn_drain`：移除 `is_end` 检查 + 提前 break，只在 `cancel` 或 `None` 时退出
- [ ] 删除 `turn_done`（语义已失效，由 Msg::XyDone 取代）

## 3. 流结束信号（mod.rs）

- [ ] `Msg` 枚举加 `XyDone` 变体
- [ ] forward task 退出前发 `Msg::XyDone`（drain drop xy_tx → recv None → 发 XyDone）
- [ ] 主循环 `Msg::Xy` 分支：移除 `is_end`→`end_stream` 逻辑
- [ ] 主循环新增 `Msg::XyDone` 分支：`app.end_stream()` + `draw_tail`

## 4. 校验

- [ ] 第 1 节复现测试转绿
- [ ] 纯文本 turn（单 TurnEnd）行为回归通过
- [ ] cancel 路径：abort 时 XyDone 正确处理（不重复 end_stream 或与 abort 分支协调）
- [ ] `cargo test --features tui` 全绿
- [ ] `cargo test --test bdd` 无回退
- [ ] `just qa` 绿
- [ ] `llman sdd validate --strict` 通过
- [ ] 同步本 tasks.md 勾选 + 归档
