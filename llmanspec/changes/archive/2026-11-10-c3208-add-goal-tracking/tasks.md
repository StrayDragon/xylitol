# Tasks

## Phase 0：SDK goal facts

- [x] 0.1 在 `AutomationFact` 追加 GoalDefined / GoalStateChanged / GoalEvaluated；复用 `custom_type = "automation"`；未知 kind 忽略。
- [x] 0.2 新增 `GoalProjection`：definition、state、iterations_used、last_verdict、last_evidence；按 branch_id 过滤。
- [x] 0.3 SDK 单测：编码/解码、投影、状态迁移、branch filter、max_iterations、未知 fact。

## Phase 1：runtime evaluator

- [x] 1.1 新增 `GoalEvaluator` 核心服务；用当前 session model 独立调用，不带工具。
- [x] 1.2 构造 evaluator prompt：acceptance + transcript + evidence；限制 acceptance 4000 字符。
- [x] 1.3 解析 met / not_met / impossible / invalid；met 无 evidence 按 not_met。
- [x] 1.4 连续 3 次调用失败/非法 -> GoalStateChanged(blocked)；不无限重试。
- [x] 1.5 evaluator 单测：fake model 四种 verdict、evidence、失败重试、blocked。

## Phase 2：DecideContinue 集成

- [x] 2.1 在 settle 阶段读取 active goal；无 goal 保持原行为。
- [x] 2.2 met -> GoalStateChanged(done) + Stop；not_met -> Continue；impossible -> failed + Stop。
- [x] 2.3 max_iterations 达到 -> blocked + Stop；iterations_used 只计 GoalEvaluated。
- [x] 2.4 pause/cancel 后不再调用 evaluator；pending 结果丢弃。
- [x] 2.5 与 c3207 engine 联动：goal active 跳过 loop tick；goal done 暂停 schedule。
- [x] 2.6 runtime 单测：Continue/Stop、max、pause/cancel、loop 交互。

## Phase 3：工具插件

- [x] 3.1 新增官方插件 `xylitol-official-plugin-goal`（或并入 automation）；配置 `goal.enabled` 缺省 true。
  - 并入 plugin-tools 的 automation 等价接线；未加 `goal.enabled` 开关。
- [x] 3.2 实现 `goal_set` / `goal_status` / `goal_pause` / `goal_resume` / `goal_clear`；schema 与 Args 手写并置。
- [x] 3.3 工具绑定 session，经 `RuntimeSessionStore` append/load facts；只写事实，不直接调用 evaluator。
- [x] 3.4 工具单测：acceptance 上限、goal_set 取代旧 goal、pause/resume/clear、错误分类。

## Phase 4：集成与行为

- [x] 4.1 MemoryStore + fake model + fake clock 集成测：goal 生命周期、loop 跳过、schedule 暂停。
  - goal 生命周期用 MemoryStore + fake model；loop 跳过与 schedule 暂停用 engine 测试；未合并为单一 fake clock 集成测。
- [x] 4.2 fork 不继承 goal；branch travel 后投影重建。
- [x] 4.3 未知 goal fact / 旧 session 不阻塞加载。
- [x] 4.4 新增 `agent-goal` BDD 场景绑定（若 specs 场景需要）；不重复单测边界。
  - specs 为裸规则，无嵌套场景，无需 binding。

## Phase 5：收口

- [x] 5.1 `llman-sdd validate c3208-add-goal-tracking --strict` 通过。
- [x] 5.2 `just qa` 绿。
- [x] 5.3 `llman-sdd change verify` 对照 design 决策表；CRITICAL 清零。
- [x] 5.4 `llman-sdd change finalize c3208-add-goal-tracking`；删除分支。
