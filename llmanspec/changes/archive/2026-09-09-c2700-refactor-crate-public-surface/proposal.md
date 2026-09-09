---
depends_on: []
skip_specs_landing: true
branch: sdd/c2700-refactor-crate-public-surface
base_sha: 43d72a54cc94d05b6e76653c1142d4b99dcfa8a9
checkpointed: true
checkpoint_sha: 43d72a54cc94d05b6e76653c1142d4b99dcfa8a9
---

# 收窄 crate 公开面：只留 embed 与 Xy*

## Why

`src/lib.rs` 已精选 `Xy*` 与 `embed`，并写明不要把 `infra::*` / `agent::capabilities::*` 当稳定面。同时 `pub mod agent` / `app` / `infra` / `protocol` / `utils` 把整层暴露给下游。发布后这会变成 SemVer 债务。Pre-0.0.1 必须一次收口：组合根仍可碰 `agent`+`infra`，外部 crate 只能走 `embed` + 根上 `Xy*`。

本 change **不改**产品可观察行为；只改编译可见性。BDD 若 `use xylitol::infra::…` 必须改走 `embed` / `XyDriver` / crate 内 `crate::`。

## What Changes

- `agent`、`infra` 改为 `pub(crate)`（或等价：子模块对外不可达）。
- `app` 除 `embed` 真正需要的类型外，不对外部暴露 `app::core::dispatch`、TUI 内部、server host。
- `protocol`：wire + ports + 根上已精选的 `Xy*` 可保持 `pub`；禁止再经 `protocol::` 深路径当第二稳定面（深路径可 `pub(crate)`，根 `pub use` 保留）。
- `utils` 保持 crate 内叶；不升格 `Xy*`。
- 测试与 `embed` 文档示例改为只依赖公开 seam。
- 删除「已知泄漏」文档里仍鼓励命名 `AgentRuntime` 的路径（改走 `into_runtime` / `into_driver`）。

## 非目标

- 不拆 crate、不抽 `xylitol-domain`。
- 不改 ReAct / 线协议字段。
- 不在本 change 塌 `XyDriver` 方法表（那是 `c2710`）。

## Capabilities

无新产品 MUST。代码组织。`skip_specs_landing: true`。若 start 后发现 live spec 钉了 `xylitol::infra::` 路径，**直接编辑**该 spec 去掉路径钉死（spec 维护规则），不要为本 change 新开行为条款。

## Impact

- 外部 `xylitol` 依赖若 `use xylitol::infra::…` / `agent::capabilities` 会编译失败（预期；文档已声明非稳定）。
- 仓内 BDD / 集成测试改 import。
- 组合根（`app/core`、CLI、server）继续 `crate::agent` + `crate::infra`。

## 本批依赖

本 change 是可见性底座。下游：`c2705`、`c2715`、`c2720`、`c2735`。并行：`c2725`（不依赖本 change）。

## Further Notes

实际 diff 范围（apply 后）：约 132 个文件。可见性收窄集中在 `src/lib.rs`、`src/app/mod.rs`、`src/protocol/mod.rs`；`tests/bdd`（52 文件）整树挂载为 crate 内 `#[cfg(test)] mod bdd`（`tests/bdd/main.rs` → `suite.rs`，路径 `crate::tests::bdd::`），`tests/inner/` 收纳原独立测试与 bench（bench 转 `#[cfg(test)]` 入口）；其余为收窄暴露的「仅测试消费」死码注解（`#[allow(dead_code)]`，约 40 簇，删除登记为后续 draft change）、rustdoc 去私有链接、BDD 校验命令迁移（`llmanspec/config.yaml` run_command → `cargo test --lib --all-features tests::bdd::`）与 API 表面快照接受。`c2705` 接手时注意：`BootstrappedAgent.agent` 已 `pub(crate)`；`agent/infra/utils` 已 `pub(crate)`；`app::tui` 因 spec tt08 保持 `pub`。


审计报告：[src 发布前审计 canvas](file:///home/l8ng/.cursor/projects/home-l8ng-Projects-straydragon-xylitol/canvases/src-pre-release-audit.canvas.tsx) 候选 #2。架构 SSOT：`src/AGENTS.md` 三层契约与组合根。
