---
depends_on: []
title: ReAct 内环中间件 SPI——插件可参与 loop 决策
needs_specs_change: true
---

# ReAct 内环中间件 SPI——插件可参与 loop 决策

## Why

- 今日 rutis 插件只能**旁路**参与 ReAct：`XyHookBus` 观察/阻断，`AgentHooks` 单槽 `before_tool` / `after_tool` / `should_stop_after_turn`。**续跑判定、模型请求整形、assistant 改写**全部写死在 `run_react_loop`（`crates/xylitol-runtime/src/agent/runtime/react/mod.rs`），插件无入口。
- 结果是「插件生态是方向」落不了地：想在模型调用前注入上下文碎片、想按自己的策略结束一轮、想截断工具结果，只能改 core 或写第二套旁路语义（违反「不得旁路第二套语义」）。
- 反向约束同样重要：**不得**让插件整段替换 ReAct（单写者、取消、权限最终裁决会丢）。因此需要的是**有界参与**的 SPI，而不是可卸 loop。

## What Changes

- **SPI（`xylitol-plugin-sdk`）**：新增 loop 中间件词表与注册表服务。
  - `LoopStage` 闭集：`before_turn` / `before_model` / `after_model` / `before_tool` / `after_tool` / `decide_continue` / `after_turn`。
  - 返回值契约 `{ ctx, control }`；`LoopControl` 闭集：`continue` / `stop` / `compact` / `handoff`。
  - `LoopMiddlewareRegistry`（`register(stage, order, id, fn)` / `unregister(id)` / 有序快照），经 `Ctx` 服务提供；卸载经 rutis `Effect::Disposer` 卸干净。
  - 分阶段载荷：turn 上下文、模型请求（messages / tool schema / model 参数）、assistant、tool 调用、tool 结果、续跑上下文。
- **执行权威（`xylitol-runtime`）**：`run_react_loop` 在七个点调用有序链；core 保留 turn 边界、取消、权限最终裁决、消息落盘格式。链为空时零开销、行为与基线一致。
- **`decide_continue` 硬契约**：core 在 Settle 后按枚举裁决；插件不得自行再调模型；core 对「无进展的强制 continue」设硬上限防死循环。
- **可观测**：每次中间件调用留 `loop.middleware` span（id / stage / order / control / 改动摘要），沿用 `provider_trace_active()` 门。
- **组合根（`xylitol-kernel`）**：核心插件 `agent-react` 提供注册表服务；`Kernel::boot` 把它交给 driver → agent；每次 run 启动快照链。
- **官方示例插件**：新增 config-gated `xylitol-official-plugin-loop-example`（一个 `before_tool` 只收紧、一个自定义 `decide_continue`），证明第三方可在 rutis 生命周期内注册/卸载；未配置零装配。
- **Hooks 共存**：`XyHookBus` 与 `AgentHooks` 保持现状（旁路审计 / 兼容一期），不改语义；审批与截断的「更严」能力由 middleware 叠加在 core 权限之前。

## Out of Scope

- 让插件 `implements AgentLoop` 整段替换 ReAct（**禁止**）。
- Strategy 槽（`ReActStrategy` / `PlanThenActStrategy`）：本 change 只记录方向，**不落空壳 trait**。
- Agent team / 子 agent 舰队 / 云端开 PR / Dream / 向量索引 / Repo map。
- ACP Registry、Skills 市场、真 OS 沙箱全家桶、dylib / 插件市场 / 注册中心。
- 侧栏深做、IDE 级 Diff/hunk 审阅、美元 Cost 面板、Eval Dashboard。
- 把 MCP bridge 改成自带摘要（摘要走 `after_tool`，不进 bridge）。
- Skills 展开迁出 core 默认路径（见 design 权衡）。

## Capabilities

- `agent-runtime`：新增七阶段中间件 MUST、控制枚举硬契约、权限「只能更严」、死循环上限、trace 可指认、示例插件零配置零成本与卸载零残留。既有 ReAct 规则（thin-react-loop、iteration-close-vs-settle 等）不变。

## Impact

- crate：`xylitol-plugin-sdk`（词表 + 服务键）、`xylitol-runtime`（内环接线 + 观测）、`xylitol-kernel`（provide / boot / rows / builtins）、`xylitol-official-plugin-config`（配置字段）、新增 `xylitol-official-plugin-loop-example`。
- 门禁：`scripts/check_crate_graph.py` 登记新 crate 与 config→loop-example 的 peer 例外。
- 测试：`tests/bdd`（agent-runtime 绑定 + 步骤 + fixture 注册入口）、kernel 契约测试（注册/卸载零残留）、runtime 单测（死循环上限 / 链空零开销）。
- 文档：`src/AGENTS.md`（中间件边界指针）、根 `AGENTS.md`（扩展入口一句）、`configs/example.yaml`（示例段）。

## Further Notes

- Explore 纪要（as-is 调用图、hooks vs middleware 的缝、seam 候选）：`research/loop-extensibility-as-is.md`。
- 与 c2936（rutis 内核）一致：注册表走 `Ctx` 服务；runtime 经 `xylitol_plugin_sdk::rt` 使用 `Ctx`，不 `use rutis`。
- 「只能更严」的落地顺序：脚本 hooks → `AgentHooks` → middleware `before_tool` → **core permission 最后裁**。
