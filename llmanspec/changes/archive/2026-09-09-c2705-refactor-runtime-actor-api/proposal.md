---
depends_on:
- c2700-refactor-crate-public-surface
skip_specs_landing: true
branch: sdd/c2705-refactor-runtime-actor-api
base_sha: 43d72a54cc94d05b6e76653c1142d4b99dcfa8a9
checkpointed: true
checkpoint_sha: 43d72a54cc94d05b6e76653c1142d4b99dcfa8a9
---

# Runtime 只走 actor 面：藏 AgentCapabilities

## Why

`AgentRuntime` 是 ReAct actor，却把内部 `AgentCapabilities`（约 40 个 `inner.*` 方法）透给 `XyDriver` / embed。组合根和测试绕过 actor 直接拧模型、queue、compaction。发布后这会把「引擎内部聚合」冻成第二稳定面。Pre-0.0.1 应收成：外部只碰 Runtime 的会话/turn API；Capabilities 留在 `agent` 内。

## What Changes

- `AgentCapabilities` 改为 `pub(crate)`（或模块私有）；删除根/`embed` 对它的 re-export。
- `AgentRuntime` 成为仓内编排入口：Driver / host 只调 Runtime（或更薄的 session handle），禁止 `runtime.inner.select_model` 一类直穿。
- 今日 Driver 仍 1:1 转发的能力，本 change **先**改成 Runtime 方法（或 Runtime 内部调 Capabilities），**不**在本 change 塌 `XyDriver` trait（那是 `c2710`）。
- 测试：ReAct 单测可继续构造 Capabilities；app / embed 测只经 Runtime / Driver。

## 非目标

- 不改 ReAct 循环算法、不改 wire Command。
- 不把 Capabilities 拆成多个 port。
- 不新增 `Xy*`。

## Capabilities

无新产品 MUST。`skip_specs_landing: true`。若 live spec 点名 `AgentCapabilities` 路径，绑定分支后直接改 spec 措辞为 Runtime / Driver。

## Impact

- `BootstrappedAgent::agent` 若仍 `pub AgentRuntime`，字段/方法面收窄；embed 文档改 `into_driver`。
- Driver in-process 从 `inner.*` 改为 Runtime API；方法数暂时可仍多（留给 c2710）。

## 本批依赖

必须先 `c2700`（否则外部仍 `xylitol::agent::capabilities`）。阻塞 `c2710`。
