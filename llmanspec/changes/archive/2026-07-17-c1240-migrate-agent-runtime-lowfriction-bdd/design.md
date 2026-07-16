# Design — c1240-migrate-agent-runtime-lowfriction-bdd

## 背景

c1215 试点时把 agent-runtime spec 全部 27 个 scenario 批量标了 `feature:true`，但只手工生成了
2 个 .feature 场景（react-terminates、stream-is-xyevent）并绑定 step。其余 22 个处于「声明可
执行但未落地」的悬空状态。本变更推进低摩擦批次（5 个），让声明与实现一致。

## 决策

### 1) 策略二：调 spec.toon 对齐现有 step（补充 c1220 策略一）

c1220 固化的策略一（纯文本 step 避引号陷阱）适用于「无现成 step 可复用」的场景——新写 step，
文本取 spec.toon 字段原样。本变更引入**策略二**：当现有 step 已覆盖 scenario 语义时，优先调整
spec.toon 的 given/when/then 文本对齐现有 step 字符串，实现零新代码复用。

适用判定：
- 策略二（调 spec.toon）：现有 step 函数体的编排/断言已覆盖 scenario 语义，仅文本字符串不一致。
- 策略一（新写 step）：无匹配 step，或现有 step 语义与 scenario 有实质差异。

### 2) scenario 选定与策略分配

| scenario | 策略 | 现有 step 复用 |
|---|---|---|
| ar3 continues-after-tools | 策略二 | `mock 模型先 tool 后无 tool` + `运行 AgentRuntime` + `turn_end 事件包含 toolResult`（全复用；该 given step `_g_ar_react_setup` 自足装配 model+ws+tool） |
| ar3 done-not-turn-end | 策略二 | 同上（断言回合正常结束 = 未因 Done 提前截断） |
| ar12 abort-drops-sse | 混合 | when/then 复用现有（`经 Driver 启动会话并在首个 TextDelta 后 abort` + `事件流包含 aborted 错误`）；given 新写自足 step `已装配慢速流式 mock 模型`（策略一），因现有 `mock 模型慢速流式返回 {n} 段文本间隔 {ms} 毫秒` 依赖 agent.feature Background |
| ar4 builder-build | 策略一 | 新写 3 step：`AgentBuilder 已装配依赖` / `build` / `得到 AgentRuntime`（trivial 构造断言） |
| ar5 ports-exist | 策略一 | 新写 3 step：`检查 runtime_protocol 端口` / `SessionStore 与 EventSink` / `trait 存在且可被实现`（trivial 端口存在性断言） |

### 2a) 实施中发现：solidify feature 无 Background

solidify 生成的 .feature **每个 scenario 必须自足**（无 Background 段），这与手写 `tests/features/`
不同（后者常用 Background 统一装配 model+tools+workspace）。

由此带来的约束：solidify scenario 的 given step **必须自足完成全部装配**，不能依赖 Background。
ar12 原计划全复用现有 `mock 模型慢速流式返回 {n} 段文本间隔 {ms} 毫秒`，但该 step 仅设 fake state、
不注册 model（它依赖 agent.feature 的 Background「有一个临时工作目录 / 配置了 mock 模型 / 工具
注册表包含 7 个内置工具」三行装配）。在 solidify feature 里无此 Background，导致「no model
configured」错误。

修正：ar12 given 改用新写纯文本 step `已装配慢速流式 mock 模型`（参照 `_g_ar_react_setup`
自足模式 + `set_fake_slow_stream`）。这是策略一（新写 step）在「现有 step 不自足」场景下的应用。

**迁移决策树补充**：策略二（调 spec.toon 对齐现有 step）的前提是「现有 given step 自足」；
若现有 step 依赖 Background 装配，则退化为策略一（新写自足 step）。ar3 的 `_g_ar_react_setup`
自足（注册 model+ws+tool），故策略二成立；ar12 的 `_g_agent_mock_slow_stream` 不自足，故退化为
策略一。

### 3) ar3 两 scenario 共享相同 step 文本

ar3 的 `continues-after-tools` 与 `done-not-turn-end` 在策略二下调为相同的 given/when/then 文本。
两者都断言「回合正常结束（turn_end 含 toolResult）= 未因 Done 提前截断」。rstest-bdd 允许多个
scenario 复用同一 step 字符串，无冲突。语义上，两个 scenario 分别从「继续下一轮」和「未提前
结束」两个角度描述同一不变量，共享测试编排可接受。

### 4) 不纳入 ar13/ar14

ar13 `no-rloop`（src 无 `r#loop` 标识符）与 ar14 `no-facade`（agent/facade.rs 不存在）是源码静态
检查型，更适合 verify 阶段的 arch_guard 而非行为 BDD。留待后续讨论是否归入或转为 verify 检查。

## 关键代码事实（支撑）

- 现有 step（bdd.rs）：`_g_ar_react_setup`（2147）、`_w_ar_react_run`（2176）、
  `_t_ar_react_terminates`（2188）、`_g_agent_mock_slow_stream`（2080）、
  `_w_driver_abort_after_first_delta`（2092）、`_t_agent_aborted_error`（2112）、
  `_t_agent_turn_end_has_tool`（671）。
- AgentBuilder 装配（agent/builder.rs）+ build 返回 AgentRuntime。
- runtime_protocol 端口：`XySessionStore` + `XyEventSink` trait（已在 bdd.rs 多处用 Arc<dyn ...>）。

## 验证证据

- `cargo test --test bdd -- --test-threads=1` → 期望 105 passed（100 + 5），0 回归
- `llman sdd validate c1240 --strict` 通过
- `llman sdd validate agent-runtime --check` → 7 features parsed（2 原有 + 5 新增）

## 后续

中/高摩擦 scenario（ar8 steer/queue、ar9、ar10 abort-clears-steer/abort-cancels-bang、ar11
second-run-after-abort、ar15-ar20）留待后续独立 change，需先设计 steer/follow_up 的 step 文本
协议与 abort 多轮编排 harness。
