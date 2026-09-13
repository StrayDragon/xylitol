---
depends_on:
- c25-fix-compact-overflow-placeholder
needs_specs_change: true
---

# 紧凑窗口频繁压缩模式：地板感知触发 + no-shrink guard + 压后大小呈现 + lab 回放

## Why

手动实测（2026-09-13，Ornith-1.5-35B @ 32k 窗口）发现：c25 修复后压缩已真实瘦身并独立呈现，
但**每轮 turn-end 必然重新触发**。数值推导（真实配置 `reserveTokens: 16384, keepRecentTokens: 20000`）：

- 触发阈值 = window − reserve = 32,768 − 16,384 = **16,384**；
- 固定开销（system prompt + 工具 schema，含 MCP）≈ 9k，压缩不可消除；
- 压后保留尾 = min(keepRecent, 阈值 − 开销) ≈ 7,384；
- **压后地板 ≈ 9,000 + 7,384 + 摘要 ≈ 17,900 > 阈值 16,384** → 压完即刻超标，40 轮模拟 40 次触发。

根因：现行触发公式（c16：`tokens > window − reserve`）**没有「压后地板」概念**——当地板高于阈值时，
压缩退化为无增益 churn。而「小硬窗口 + 频繁压缩」是本产品的真实使用模式（本地量化模型 KV cache
受内存约束，用户刻意以 32k 窗口运行 256k 模型），值得作为一等模式自洽化，而非靠病态参数硬顶。

## What Changes

- **地板感知触发 + 迟滞（c16/c26 修订）**：auto compact 的有效触发阈值改为
  `max(window − reserve, 压后地板估计 + 迟滞带)`；地板 = 固定请求开销（c16 同源折算）+ 压后保留尾 +
  摘要占位。效果：保留「频繁」，但每次压缩必须买到真实 headroom（如较当前下降 ≥ 迟滞带），
  消除「压完即超标」空转。manual/force 不受此阈值约束（显式要求必须执行）。
- **no-shrink guard + 一次性可行动提示**：压后投影 ≥ 当前 tokens − margin 时跳过本轮 auto compact，
  并向用户呈现一次性诊断（建议：调低 keepRecentTokens / 调高 contextWindow；落点倾向 ScrollNotice，
  对齐 atc20「失败与诊断 MAY 写滚动提示」，propose 阶段定稿）。
- **压后大小呈现**：compaction 块词形从 `Compacted from N tokens` 扩展为
  `Compacted from N → M tokens`；M 取 AfterCompaction settlement 占位（c26 数据已有，纯呈现），
  让压缩有效性一眼可验（本轮实测 35,840 → ~18k）。
- **lab 回放不变量 harness**（`lab_` 形态，人跑维护，不进门禁）：以真实 session JSONL 为 fixture
  （babb2fbc / 92fa9adf）回放压缩管线 N 轮，断言：每次 compact 后投影单调下降、切点合法
  （c8）、摘要规模有界、上下文收敛——作为触发语义修订的防回归与「逻辑正确」的实证手段。

## Capabilities（预估，propose 阶段定稿）

- `domain-compaction`（c16 触发阈值地板协调、c26 压后大小数据、guard 早退语义）
- `app-tui-transcript` / `app-tui-bridge`（块词形 N → M）
- 测试：`tests/lab_compaction_replay`（lab）+ 既有单测扩展

## Open Questions

- 迟滞带取值：固定比例（如地板 +25%）还是可配置（`compaction.hysteresis`）。
- no-shrink 提示落点与词形：ScrollNotice（一次性）vs loaded-resources 卡诊断行（obs_diag 同款）。
- 块词形 N → M 的 spec 落点：att29 邻域修订还是 bridge compaction-status 场景。
- guard 是否也要拦住 overflow 路径的 retry（overflow Case1 已请求失败时，压缩是失败恢复手段，
  可能不应被 guard 拦截）。
- lab fixture 的脱敏方式（真实 JSONL 内含用户代码片段）。
