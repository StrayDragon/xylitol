---
depends_on:
  - c2945-update-loop-middleware-payloads
title: Loop 中间件批次预演的停止语义入合约
branch: sdd/c2951-update-loop-middleware-preflight-stop-semantics
base_branch: main
base_sha: 9faa17b40c9ab4e82df33b9835214a919dffdae9
---

# Loop 中间件批次预演的停止语义入合约

## Why

c2945 把 `before_tool` 门禁提到工具批规划之前，以便用最终工具名重算权限与并发窗。这个「整批先预演」的实现带来了一个插件作者必须知道的后果：

> 本批每个调用的 hooks 与 `before_tool` 都在任何工具执行之前跑完。若前序调用返回 `stop` 结束本 run，后续调用的中间件**已经运行过**，但它们对应的工具**不会执行**。

该语义已写进 c2945 的 design §3.3，但**没有进入现行产品合约**：`r2922` 只描述预演顺序、改参改路由、重裁与重分类，没有说明「放行不等于执行」；`src/AGENTS.md` 同样只写「在工具批规划前预演」。

后果是插件作者会按「我放行 ⇒ 工具会执行」写逻辑（例如计数、审计、资源预留、幂等标记），在批内前序 `stop` 时被静默打破。这是可观察的行为差异，属于产品合约缺口，不是内部实现细节。

## What Changes

- `r2922` 增加预演的停止语义：批内预演先于任何工具执行；前序调用返回 `stop` 时，后续调用的中间件**已运行**但其工具**不执行**；中间件不得假设「放行即执行」。
- 新增一条可执行 BDD 场景：注册两个 `before_tool` 中间件，前者对首个调用 `stop`、后者记录是否被调用；断言后者**被调用过**、且其后工具未执行（无对应 `ToolExecutionStart` / `ToolExecutionEnd`）。
- `src/AGENTS.md`「Loop 中间件」段补一句同一语义的实现边界说明（指向 spec）。

## Out of Scope

- 改变预演顺序本身（hooks → AgentHooks → 中间件 → 权限）——那是 c2945 的既定行为。
- 让中间件「预知」本批是否会被前序 `stop` 中断（不引入回滚或 dry-run 二次调用）。
- 引入「预演结果可撤销」机制（不做事务性工具批）。

## Capabilities

- `agent-runtime`：`loop-middleware-tool-gate` 补预演停止语义 + 场景。

## Impact

- spec：`llmanspec/specs/agent-runtime/agent-runtime.feature` 的 `@req:r2922` 规则句与新增场景。
- 测试：`tests/bdd/steps_loop_middleware.rs` + `bindings_loop_middleware.rs` 新增 1 个场景。
- 文档：`src/AGENTS.md`「Loop 中间件」段一行补充。
- 代码：**无**。现有实现已符合该语义（`prepare_call` 整批先行 + 逐调用 `stop_run` 传播），本 change 只把既成行为写进合约并加测试钉住。

## Further Notes

来源：c2945 design §3.3「预演带来的已知语义变化」。该项在 c2945 归档时被识别为「已入 design 但未进公开合约」，故独立成 change 补齐，避免直接编辑 live spec 绕过生命周期。
