---
id: c255-consolidate-trust-and-session
title: "信任机制单一 SSOT 落地 + AgentSession 协作对象解构"
depends_on: []
---

## Why

两项独立调查发现的架构债,合并为一次原子重构以避免漂移。

### 面积一:TRUST —— 双份死代码

调查证实 `src/agent/trust/`(store.rs + resolve.rs,~925 行,顶部 `#![allow(dead_code)]`)与 `src/infra/trust/`(mod.rs,~372 行)**均为 100% 死代码**:无任何生产调用点(`rg 'agent::trust|TrustManager'` 排除自身后零命中)。运行时唯一的"信任"状态是 `infra/settings/manager.rs` 上一个裸 `project_trusted: bool` 字段,仅由自己的单测切换。`commands.rs` 的 `("trust", ...)` 条目同样是死桩。`security-policy` capability 的 13 条需求(r1–r8, s10–s14)无一涉及信任——坐实信任特性从未接线。

两套互不相干的脚手架(store+lock 派 vs manager+inheritance 派)同时存在是纯负债,且 `infra/trust/` 的设计(pi 风格的 `{canonical_path: bool|null}` + 父目录继承)是正确的方向。

### 面积二:SESSION —— god object

`src/agent/session.rs` 已膨胀至 **1245 行 / 58 个 pub fn / 23 个内部 section / 15 个字段**。`c240-consolidate-architecture` 在 `architecture/ar02` 中确立了"小组件必须内联"的规则,但**只设了下限、未设上限**——这正是 `session.rs` 无界增长的根因。`as30`(cohesion-decomposition)已授权分解,`as31`(facade-retained)要求保留公共 API,但均未落地到具体协作对象。`session_io.rs`(97 行,其中 `stats()` 是返回 0 的空桩)是半截子抽取。

### 为何合并为单一变更

trust 接线(t5)会修改 `SettingsManager` 与 agent 启动流程;session 解构会重组 `AgentSession`。两者都触及 agent 启动路径与 `infra/settings`,分批做会产生中间漂移状态。合并为一次"锁定基线 → 重构 → 验收"保证最终态自洽。

## What Changes

1. **`architecture/ar02` 修订**:为"小组件必须内联"补一条对称的"上限"子句——内联块超过 ~100 行、有独立行为逻辑、概念内聚时,**MUST** 提取为具名协作对象;`AgentSession` facade 的单结构/单文件**MUST NOT**无界增长。

2. **trust 单一 SSOT 落地**(新增 `security-policy` t1–t6):
   - **t1**:删除 `src/agent/trust/` 整目录;信任 SSOT 唯一位于 `infra/trust/`。
   - **t2**:持久化沿用 pi 风格 `{canonical_path: bool|null}` JSON + 父目录继承。
   - **t3**:固定优先级的解析管线(override → no-inputs → store → default policy → UI prompt → fallback deny),返回决策 + 原因。
   - **t4**:UI prompt 通过 `on_prompt` 回调注入,`interface/` 负责,`agent/`/`infra/` 保持 UI-agnostic。
   - **t5**:启动时解析的信任决策门禁 project-scoped settings;移除 `SettingsManager` 的裸 `project_trusted` bool 切换,改为接入解析结果(单一 SSOT)。
   - **t6**:`commands.rs` 的 trust 命令接线到 `infra/trust/` store。

3. **AgentSession 协作对象解构**(新增 `agent-session` as32):
   - 提取 `AutoRetryEngine`、`BashExecHandler`、`SessionExporter` 三个具名协作对象,`AgentSession` 组合它们(落地 `as30`,受 `as31` 约束保留公共 API 与事件流)。
   - `session.rs` 升级为 `session/` 目录,子模块 `io` / `retry` / `bash_exec` / `export` / `stats` / `prompt` / `steering`;`session_io.rs` 并入 `io.rs` 并修复空桩 `stats()`。
   - `AgentSession` 单文件降至 600 行以下。

## Capabilities

- **`architecture`**(modify `ar02`):补组件粒度上限,使协作对象提取合法化。
- **`agent-session`**(add `as32`):点名三个协作对象 + session 目录重组。
- **`security-policy`**(add `t1`–`t6`):trust 单一 SSOT、解析管线、持久化、UI 回调、状态接线、命令接线。

## Impact

- **规模**:中-大。涉及删除 ~1300 行死代码、新增 trust 管线(~400 行)、重组 1245 行 session 为目录 + 提取 3 协作对象。
- **行为**:
  - trust:从"无"变为"有"——首次接线信任门禁(影响 project-scoped settings 加载)。这是预期的新行为。
  - session:**零行为变更**——公共 API 与事件流不变(承 `as31`),仅内部重组。
- **风险**:中。trust 接线触及安全语义(t5 门禁)与 agent 启动路径;session 解构是 1245 行的大规模搬运,依赖快照基线兜底。采用三阶段方法论(基线 → 重构 → 验收)控制风险。
- **不做**:
  - 不生产 `c240` 评估的宏代码(`#[tool]`/`#[command]`/`#[provider]`)。
  - 不改 session 持久化文件格式(`infra/session/` 不动)。
  - 不动 provider 接口、agent loop 行为、compaction 算法。
  - trust 持久化**沿用 pi 风格**,后续体验改进留待 future change(见 future.md)。

## 方法论

三阶段(对应 tasks.md 分组):

1. **锁定基线**:全绿快照归档;为 trust 死代码路径补特征化测试(将被接线);为 `AgentSession` 58 方法补行为快照(insta);记录当前 `just qa` 全绿状态作为回归锚点。
2. **重构**:`ar02` 改 → trust 实现+接线(删双份死代码,`infra/trust/` SSOT)→ session 目录重组 + 提取 3 协作对象(全程保公共 API)。
3. **验收**:`just qa` 全绿、快照稳定、`rg 'agent::trust'` 零命中、`session.rs < 600` 行、BDD 全通过。
