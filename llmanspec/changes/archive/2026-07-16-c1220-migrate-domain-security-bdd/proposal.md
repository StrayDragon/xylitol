---
change_id: c1220-migrate-domain-security-bdd
title: "BDD-on 迁移：domain-security 首迁（network-domain-block）"
status: proposal
priority: 1220
apply_band: P3-core
depends_on:
  - c1215-add-test-bdd-solidify-pilot
author: agent
track: R
wave: bdd-migration
---

# c1220-migrate-domain-security-bdd

## Why

c1215 建立了 solidify 链路试点（agent-runtime），验证了 `spec.toon → solidify → bdd.rs`
模式可行。本变更是该模式的**首次复用**：迁移 domain-security spec 的一个活行为场景到
solidify 链路，确认模式可复制、可规模化。

domain-security 是评估中摩擦最低的候选：现有 `sandbox_bdd` mod（bdd.rs 2268+）已有
XyPermission 的 step 实现，语义同构。

## What Changes

1. **delta `domain-security` modify r7**：给"网络域强制"requirement 补 BDD 可执行声明，
   新增 op_scenario `network-domain-block`（feature:true）。
2. **solidify 生成** `llmanspec/specs/domain-security/domain-security.feature`（1 场景）。
3. **bdd.rs `sandbox_bdd` mod**：新增 2 个纯文本 step（given `blocked_domains 含 evil.com`、
   when `检查网络域名 evil.com`）+ 1 个 `#[scenario]` 绑定（name=英文 id）。

## 关键发现：引号陷阱（迁移模式固化）

rstest-bdd 的 `{name:string}` 占位符**期望带引号的值**（现有 feature 写 `"evil.com"`），
但 solidify 从 spec.toon 字段**原样输出，不加引号**。故迁移场景的 step 文本**应用纯文本
（无占位符）**，避免占位符匹配失败。这是 c1215 试点未覆盖的摩擦点，本变更固化为此规则。

## Capabilities

- `domain-security` — modify r7，加 network-domain-block 可执行场景

## Impact

- 测试：新增 1 场景（99 → 100 passed），旧 sandbox.feature 链路零回归。
- 不删旧 `tests/features/sandbox.feature`（双轨并存）。
- 为后续批量迁移（agent-tools 等）固化"纯文本 step 避引号陷阱"模式。
