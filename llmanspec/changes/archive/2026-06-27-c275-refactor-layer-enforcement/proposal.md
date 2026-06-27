---
depends_on:
  - c260-refactor-domain-architecture
---

# c275-refactor-layer-enforcement

> **状态**：设计提案（2026-06-27）。本变更是 c260 分层重构的"护栏修复"——不新增功能，只把
> 名义上的硬约束（HC-1）固化为可执行的红线，并归位一个名实不符的 agent 子模块。

## Why

c260 确立了六条硬约束（HC-1…HC-6）和目标模块树，c265/c270/c271/c272 已陆续归档。但一次全量
review 暴露了三处"约束有名无实"：

1. **arch_guard 存在重大盲区，给出虚假安全感。** `src/tests.rs::arch_guard` 只 grep 四个具体
   provider（`infra::provider::{openai,anthropic,fake,mock}`），漏检 `infra::provider::factory`、
   `infra::tools`、`infra::session`、`infra::event`、`infra::resource`、`infra::sandbox` 等。实测
   `agent/` 生产代码有 **46 处** `crate::infra::` 直接导入，其中最严重的
   `agent/model/manager.rs:49` 在 `pub fn build_current_model()` 内直接调用
   `infra::provider::factory::build_provider`——provider 装配发生在 agent 内部，正是 HC-1
   明令"装配在 server/cli 组合根发生"的直接违反。守卫却全绿。
2. **`layer-architecture` spec 自相矛盾。** `la2` 宽泛要求"agent 不 import 任何 infra 具体类型"，
   但 `la9` 把守卫范围写死成 4 个 provider。守卫忠实地执行了窄规范，于是规范自身的矛盾成了
   违规的保护伞。
3. **`agent/auth` 名实不符且越界。** 模块叫 auth，实际内容（`guidance.rs`）全是**静态用户提示
   文案**（`get_provider_login_help` / `format_no_api_key_found_message` 拼字符串），不做任何认证。
   它只依赖 `core::model::ModelKind`，唯一消费方是 `interactive/cli`。按 c260 词汇表，agent 必须
   薄（只循环 + hook + 编排状态），纯 presentation 文案不该住在 agent。

## What Changes

### P1 — 收紧 arch_guard（修 la2/la9 矛盾）

- `src/tests.rs::arch_guard::agent_does_not_import_concrete_providers` 重写：
  - 全量扫描 `src/agent/` 中**生产代码**的 `crate::infra::` 导入（剔除 `#[cfg(test)]` 区域与注释）。
  - 引入**存量白名单**（`existing_violations: &[(file, needle_substring, follow_up_change)]`）：
    把当前 46 处已知违规逐项登记，每项标注负责清理的后续 change id（c276/c277/c278）。
    白名单内豁免，白名单外的新增违规定为失败。
- 这是工业界"技术债护栏"做法（error-on-new, warn-on-existing）：立即可防新增违规，存量分批修。

### P2 — auth 引导归位 + 改名（la13）

- `agent/auth/` 整体迁出：`guidance.rs` → `interactive/cli/`（内联或 `cli/provider_guidance.rs`）。
- 删除 `agent/auth/` 与 `agent/mod.rs::pub mod auth`。
- 改唯一调用方 `interactive/cli/mod.rs:7` 的 `use crate::agent::auth` 为本地引用。
- 文案函数名保留（`user-experience` spec ux1–ux4 要求这些函数存在，行为不变），仅迁移位置。

### P3 — 文档修正（清腐化）

- `AGENTS.md`：层列表补 `server/` 模块；`protocol/` 表述与实际单文件 `protocol.rs` 对齐（或反之）。
- `src/server/mod.rs:7`：删 "Currently under construction via c270" （c270 已归档）。
- `src/tests.rs` arch_guard 头注释：删 "tracked for cleanup by c270 / T2 c270" 等过期 TODO。
- `agent/facade.rs`：清 "future RemoteDriver" 注释（`server/ws.rs` 已实现 ServerFrame/ClientFrame）。

### P4 — 填白名单 + 验证

- 将 review 产出的 46 处违规清单（按文件聚合）写入 `arch_guard` 白名单，每项标后续 change id。
- 全量 QA：build + nextest + BDD + arch_guard 全绿。

## Capabilities

- `layer-architecture`（modify）：`la9` 守卫范围扩大到全量 `crate::infra::`；新增 `la12`（白名单
  机制）、`la13`（agent 不持 presentation 文案）。

> `user-experience`（ux1–ux4）与 `cli-entry`（ce1）**不动**：前者需求不绑定实现位置，后者已覆盖
> cli 的依赖隔离。

## Impact

- **可执行红线**：arch_guard 从"4-provider 窄检查"升级为"全量 + 白名单"，任何新 agent→infra
  耦合立即被 CI 拦截。这是后续 c276/c277/c278 重构的安全网。
- **模块瘦身**：删除 `agent/auth/`，agent 子目录 8 → 7；消一个名实不符。
- **零行为变更**：auth 文案函数签名/返回值不变，仅迁移；BDD 88 场景不受影响。
- **后续变更路线图**（写入 design.md，本变更不实现）：
  - `c276`：词汇上提 core（`SessionEntry` 族 / `SkillInfo` / `SourceInfo` / `CompactionSettings`）——消白名单约 30 项类型性耦合。
  - `c277`：装配下沉组合根（`build_provider` / `default_tools` / `SessionManager` / `EventBus` / `SandboxEngine` 注入）——消剩余约 10 项。
  - `c278`：`agent/session` 瘦身 + `export` 转发层合并。
