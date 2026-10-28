---
depends_on: []
title: Loop 拓扑策略槽的决策：不引入 ReActStrategy trait，先用中间件组合验证
needs_specs_change: false
branch: sdd/c2956-add-loop-strategy-decision
base_branch: main
base_sha: 34bf3a6133043c27543547d55f960d49c514b6cd
---

# Loop 拓扑策略槽的决策：不引入 ReActStrategy trait，先用中间件组合验证

## Why

原始 Loop Middleware 指示里有一个可选槽：

> 同一时间仅一个 `ReActStrategy` / 未来 `PlanThenActStrategy`；细节仍走 Middleware。默认实现 = 今日行为。

`c2944`–`c2953` 已把七阶段中间件落地并归档，但**没有**引入 `ReActStrategy`。这不是遗漏，而是需要把「为什么不做、什么条件下才做」写成可复核的决策，避免后续把「Strategy 槽」误读成「插件可整段替换 ReAct」。

现状证据：

- 全仓无 `ReActStrategy` 概念（`rg ReActStrategy crates plugins src` 为空）。
- 七阶段已覆盖 `before_turn` / `before_model` / `after_model` / `before_tool` / `after_tool` / `decide_continue` / `after_turn`，且 `decide_continue` 的 `control` 闭集为 `continue | stop | compact | handoff`。
- `handoff` 未支持时以稳定 `LoopMiddlewareHandoffUnsupported` 报错，是未来拓扑的预留门。
- 插件是 in-process Rust（rutis）；非 Rust 能力只能经 MCP / shell hooks / Skills 进入。

## What Changes（候选，promote 时逐条重估）

1. **决策记录**：不新增 `ReActStrategy` trait；loop 拓扑 = 默认 ReAct + 中间件组合。Strategy 槽的语义收敛为「用中间件 + 配置表达不同拓扑」，不新增第二套改 loop 行为的方式。
2. **低成本实验**：先在官方示例插件里做 plan-as-state 中间件（见 `c2957`），验证「不换 runtime 也能 Plan-then-Act」；用实测结果决定是否还需要 core 级拓扑 seam。
3. **触发条件**：只有同时满足以下三条，才考虑 core 级策略槽（且优先做数据/配置驱动的模式，而不是 Rust trait）：
   - 出现中间件表达不了的拓扑（例如需要 core 并行发起 N 次 model call 的 Parallel Exploration，或需要跨 resume 的 plan DAG 执行器）；
   - 至少两个独立实现/消费者；
   - plan-as-state 实验实测失败（plan starvation / replan thrash / 上下文压不住）。
4. **文档落点**：决策摘要进 `src/AGENTS.md` 的 loop 边界段（稳定边界），细节留本草案与 `c2957` 的 research/实测记录。

## Capabilities

- `agent-runtime`：本草案不改行为合约；若 promote 为文档决策，只补稳定边界指针，不新增 MUST。
- 未来若真做 core 策略槽，再单开 change 写 `agent-runtime` 的拓扑选择合约。

## Impact

- 代码：本草案零代码；`c2957` 是纯插件示例，不动 core。
- 文档：`src/AGENTS.md` 增加一句「loop 拓扑 = ReAct + 中间件组合；Strategy 槽不是插件替换 ReAct 的入口」。
- 测试：`c2957` 的单测/BDD 提供 plan-as-state 的行为证据。

## Further Notes

### ROI 判断（为什么现在不做 trait）

| 维度 | 判断 |
|---|---|
| 谁能实现 | 只有 in-process Rust 插件；非 Rust 插件无法实现 Rust trait |
| 消费者 | 0 个；今天没有第二个拓扑的真实需求 |
| 与 Middleware 的关系 | 七阶段已覆盖决策点；再加 trait 会出现两套改 loop 行为的方式，测试矩阵翻倍 |
| Rust 增量 | trait + 注册表 + 生命周期 + 冻结快照 + 文档 + BDD，约 300–600 行 core |
| 与产品目标 | 直接违背「尽量少写 Rust、能力外置、用其他语言/运行时补」 |

### ADPS 模式的 ROI 映射（一手来源）

参考 `huangjia2019/agent-design-patterns`（27 矩阵模式 + 3 横切面 + 5 扩展 + 1 候选）。其选型法明确要求「收束到 3–7 个模式」，扩展模式晋升还需满足五项条件（反复出现、结构稳定、可观察失效、边界清楚、多个独立实现）。

| 组 | 代表模式 | xylitol 落点 | 结论 |
|---|---|---|---|
| 已覆盖，零 Rust | P2 语义压缩、M1 分层保留、M3 进度追踪、F2 技能包、A3 提示链、A4 护栏三明治、G1 审批门、G4 钩子管线、M5 程序性记忆、R1 思维链 | core compaction / AGENTS / Skills / hooks / permission / provider thinking | 保持 |
| 插件可做，零 core | A5 最小工具集、A2 规划执行、M4 失败日记、F4 自愈循环、F1 生成评审、R2 复杂度路由 | `before_model` 改 schema/options、`after_tool`/`after_turn` 写文件、`decide_continue` 续跑 | **优先** |
| 需 core / 新拓扑 | R3 并行探索、A2 一等公民 Plan DAG 执行器、G2 爆炸半径、G3 渐进承诺、R5 交互者-推理者 | 需要 core 调度/策略引擎/能力账本 | 暂缓，先证明中间件不够 |
| 非目标 | C1–C5 多 Agent、C6 编舞、M2 RAG/向量索引、P4 多模态 | — | 不做 |

关键结论：**高 ROI 的模式恰好都落在 Skills / MCP / hooks / middleware；高成本模式恰好都集中在多 Agent / 新拓扑**。这支持「薄 core + 厚插件面」，不支持先造 Strategy trait。

### Plan-and-Execute 的关键证据

ADPS A2 的参考实现明确写：

> "The DAG is intentionally trivial… **The runtime is plumbing; the contract is the pattern.**"

也就是说，Plan-and-Execute 的核心是**把 plan 变成持久工件**（`PlanStep` 的 `depends_on / inputs / outputs / verification / rollback`），由 Orchestrator 维护状态；不是换一个 loop 引擎。Claude Code 的 plan-as-file、Manus 的 `todo.md` 都是同一路子。

### 已知风险 / 留待

- `DecideContinueCtx` 不暴露 follow-up 队列状态；plan 驱动的 `stop` 会按 `r2921` 跳过 follow-up。若 plan 模式产品化，需要 core 暴露队列状态或在 core 侧仲裁（本草案不预设做法）。
- `c2957` 的实验只证明「中间件组合能驱动 plan 状态」，不证明长程 plan 的 checkpoint / resume / 局部 replan 已达标；后者若要做，单开 change。
