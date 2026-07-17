---
change_id: c1215-add-test-bdd-solidify-pilot
title: "BDD-on solidify 链路试点：agent-runtime 首迁"
status: proposal
priority: 1215
apply_band: P3-core
depends_on: []
author: agent
track: R
wave: bdd-migration
---

# c1215-add-test-bdd-solidify-pilot

## Why

0.0.61 的 BDD-on 模式以 `spec.toon` scenarios 为 SSOT，经 `llman sdd solidify` 生成
`llmanspec/specs/<capability>/<capability>.feature`（场景标题 = `scenario.id`）。但本项目当前
BDD 全部走旧链路：手写 `tests/features/*.feature`（中文场景标题）+ `tests/bdd.rs` 绑定。两套链路
的标识空间与路径都不同，需逐步迁移对齐 SSOT。

本变更是迁移的**首个试点**，验证 `spec.toon → solidify → bdd.rs` 新链路在本仓库可行，并建立
后续逐 spec 迁移的模式。试点已实现并通过（99 passed），本变更把它正式化为 SDD 合约。

## What Changes

1. **新增 `test-bdd` 合约 tb3**：BDD 场景 MUST 支持 solidify 链路（`#[scenario]` name = `scenario.id`），
   且 MUST 与现有 `tests/features/` 手写链路并存、互不干扰。
2. **新增 `agent-runtime` 合约 ar-pilot**：在 `llmanspec/specs/agent-runtime/` 携带试点 .feature，
   含 `react-terminates` 与 `stream-is-xyevent` 两个 solidify 风格场景。
3. **实现已就位**（本变更 apply 时确认）：
   - `llmanspec/specs/agent-runtime/agent-runtime.feature`（solidify 风格生成）
   - `tests/bdd.rs`：6 个 step + 2 个 `#[scenario]` 绑定（path 指向新位置，name 用英文 id）

## Capabilities

- `test-bdd` — solidify 链路 BDD 合约（tb3）
- `agent-runtime` — 试点 .feature 存在性（ar-pilot）

## Impact

- 测试：新增 2 个 solidify 风格场景（97 → 99 passed），旧链路零回归。
- 不删除任何现有 `tests/features/` 文件或绑定（双轨并存）。
- 不改 config.yaml（`feature_dir` 已确认不被 solidify/validate 读取）。
- 为后续逐 spec 迁移（B 阶段）建立可复用模式。
