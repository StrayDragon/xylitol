# Design

## Evidence

- `plugins/xylitol-official-plugin-plan/src/middleware.rs:75-84`：`load_mode` → `store.load_leaf_branch(session_id)` → `latest_plan_mode(&branch)`。
- `crates/xylitol-runtime/src/agent/runtime/react/mod.rs:1122`：状态栏 `AgentStatusBar::from_session_entries(&branch)` 又读一次 leaf branch。
- `llmanspec/specs/agent-runtime/agent-runtime.feature:454`（`r2955`）：中间件须在 `before_llm_request` 读取最新 `plan_mode`。
- `llmanspec/specs/agent-plan/agent-plan.feature:10`（`r2948`）：切换须立即写入状态，并在下一次模型调用生效。

## Options

### Option A：core 读一次，ctx 传给中间件（推荐）

- core 在每次迭代读取一次 leaf branch，得到 `latest_plan_mode`；
- 把 `plan_mode: Option<PlanMode>` 放进 `BeforeLlmRequestCtx`；
- 状态栏复用同一结果；
- plan 中间件不再直接读 store。

优点：一次读取；无陈旧缓存（每迭代重读）；语义仍是“最新”。
缺点：`BeforeLlmRequestCtx` 增加字段（SDK 变更，需 `SDK_API_VERSION` bump）；所有中间件构造点要补字段（可用 `..Default` 或构造 helper）。

### Option B：store 增加尾部查询

- 新增 `load_latest_custom_entry(session_id, custom_type)`，只读 active segment 尾部；
- 中间件与状态栏各查一次，但每次数据量小。

优点：不动 SDK ctx；读取成本下降。
缺点：仍是两次查询；需要 store 实现与测试；跨 segment 的最新条目要正确回退。

### Option C：中间件缓存

- 缓存 `PlanMode`，按 `session_id + leaf_entry_id` 失效；
- 优点：零 SDK 变更。
- 缺点：其他客户端/路径改变模式时可能陈旧；需要失效钩子；与“读取最新”语义有偏差。

### Option D：接受现状

- 优点：零改动。
- 缺点：长会话双读开销持续。

## Recommendation

采用 **Option A**：

- core 已经为状态栏读一次，直接把结果传给中间件最自然；
- 每迭代重读，不存在陈旧问题；
- 代价是一次 SDK 字段新增，可在下一次 SDK 版本 bump 时一并处理。

若不愿动 SDK，退而求其次选 **Option B**。

## Test seam

- 单测：fake store 记录 `load_leaf_branch` 调用次数；跑一个迭代，断言从 2 次降到 1 次。
- 行为测试：切换 `plan_mode` 后下一迭代中间件看到新模式（不得因缓存而延迟）。
- 长会话基准：可写一次性 `lab_` 脚本测量读取耗时，不进 `just qa`。

## Risks

- `BeforeLlmRequestCtx` 字段新增会影响插件编译；需要 SDK 版本 bump 与迁移说明。
- core 读取失败时（store 不可用）须降级为 `None`，中间件不得中断 run（对齐 `r2955`）。
- 状态栏与中间件共用同一份数据，需确保两者对“无 plan_mode”的默认值一致（execute）。

## Open questions

1. 选 A（ctx 传值）还是 B（store 尾部查询）？
2. 是否借本次一并 bump `SDK_API_VERSION`？
3. 读取失败时的降级值：`None` 还是 `PlanMode::execute()`？

## Decision (implemented)

- 采用 Option A：core 每迭代只读一次 leaf branch，`BeforeLlmRequestCtx.plan_mode` 传给中间件。
- plan 中间件删除 `load_mode` / store / ctx 字段，缺失时降级 execute。
- `SDK_API_VERSION` → `0.5.0`（后续 c2980 再 bump 到 `0.6.0`）。
