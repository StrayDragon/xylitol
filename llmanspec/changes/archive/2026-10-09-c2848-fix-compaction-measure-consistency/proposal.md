---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 压缩度量口径一致与校准（消除三处估算分歧 + 展示/决策矛盾）

## Why

实机深会话（0664dab6，189 条）同一时刻三处 token 度量**互不同源**：

| 用途 | 实现 | 观测 |
|---|---|---|
| 切点/守卫 | `estimate_tokens_entry_for_cut`（chars/4 字符串启发式） | 「连最旧都该保留」（低计） |
| 触发/footer | `estimate_from_session_entries`（tokenizer 编码 + fixed_context） | 35k（≈2x 高估） |
| 真实 | provider 服务端计数（llm.request input） | **17.4k** |

后果：footer 显示 `106.4%/33k` 超窗，压缩却判定「无可压缩」永不触发，每次 turn-end 重复判定双发 `compaction.skipped`；真实上下文实际 17.4k 远未触碰窗口（安全但展示与决策矛盾、无法解释）。**若真实上下文接近窗口，chars/4 低估会让 keep 段被算小，压缩后真实量可能仍在窗口边缘**。r1406 已约束「触发判定与 footer 同源、MUST NOT 用 len/4」，但**切点/守卫路径仍游离于该纪律之外**——本轮把度量一致性纪律覆盖到全决策链并做校准对拍。

## What Changes

1. **切点/守卫度量并入统一入口**：`prepare_compaction` / `find_cut_point` 的 token 度量改走与触发/footer 相同的估算源（token_estimator 单入口），消除 chars/4 独立启发式作为决策 SSOT；保留 lax 兜底但 MUST NOT 作为决策默认。
2. **校准对拍（先只读 lab）**：对 0664dab6 深会话用「统一估算 vs provider 实测 input」出数，定位 2x 分歧来源（tokenizer 口径 / fixed_context 叠加 / 历史去重），确定统一后度量与 provider 的偏差边界。
3. **spec `domain-compaction` r1924**：度量一致性纪律覆盖切点判定（同源 sin 单入口），并对「产品估算 vs provider 实测」的系统性偏差可解释。
4. **回归守卫**：锁「同一切换后上下文上，切点判定、触发、footer 三处度量一致」（BDD/单测）；可选 lab 校准对照（不进 qa）。

**范围附注（用户暂缓）**：turn-end `compaction.skipped` 双发（overflow + threshold 双入口各评估一次）**本轮不重构**；口径统一后若噪音仍扰人再单独处理。

## Capabilities

- `agent/compaction`（度量口径）、`protocol/metrics`（估计结构）、`test-infra`（校准对照）

## Impact / 风险

- 切点判定口径变化会影响**实际压缩触发时机**（更贴近真实窗口）——属预期行为变化，需 BDD 覆盖既有切点用例（cut_detector 测试面）。估算输入含 tokenizer override 时 CPU 成本高于 chars/4（spawn_blocking 隔离，注意切点热路径）。
- 若校准发现 2x 高估源于 fixed_context 被重复计入，需同时修正 EstimateContext 与触发路径，范围可能扩到 `app/core` 估算管线。
