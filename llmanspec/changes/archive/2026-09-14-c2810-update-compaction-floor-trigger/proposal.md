---
depends_on:
- c25-fix-compact-overflow-placeholder
needs_specs_change: true
branch: sdd/c2810-update-compaction-floor-trigger
base_sha: bcb3791043d0f047f134f453883d80a2d2e10faf
base_branch: main
---

# 紧凑窗口频繁压缩：地板感知触发 + 压后大小呈现 + 一次性诊断 + lab 回放（c2/c26 修订）

## Why

手动实测（2026-09-13，Ornith-1.5-35B @ 32k 窗口）发现：c25 修复后压缩已真实瘦身并独立呈现，
但**每轮 turn-end 必然重新触发**。数值推导（真实配置 `reserveTokens: 16384, keepRecentTokens: 20000`）：

- 触发阈值 = window − reserve = 32,768 − 16,384 = **16,384**；
- 固定开销（system prompt + 工具 schema，含 MCP）≈ 9k，压缩不可消除；
- 压后保留尾 = min(keepRecent, 阈值 − 开销) ≈ 7,384（c8 clamp 已生效）；
- **压后地板 ≈ 9,000 + 7,384 + 摘要 ≈ 17,900 > 阈值 16,384** → 压完即刻超标，40 轮模拟 40 次触发。

根因：现行触发公式（c2：`tokens > window − reserve`，pi 同构）**没有「压后地板」概念**——当地板
高于阈值时，压缩退化为无增益 churn。「小硬窗口 + 频繁压缩」是本产品真实使用模式（本地量化模型
KV cache 受内存约束，用户刻意以 32k 窗口运行 256k 模型），值得作为一等模式自洽化，而非靠病态
参数硬顶。

## What Changes

- **地板感知触发（c2 修订）**：threshold auto 的有效触发阈值改为
  `max(window − reserveTokens, 压后地板 + 迟滞带)`。压后地板 = 固定请求开销（c16 同源折算）
  + 有效保留尾（c8 clamp）+ 摘要占位（最新 `CompactionEntry.summary` 实测 chars/4，无则保守
  常量）；迟滞带 = 压后地板 × 25%（固定常量，不加配置）。开销未知/未注入时退化回
  `window − reserveTokens`（与 c8 clamp 同款退化纪律）。效果：32k 实测场景阈值升至 ≈23k，
  每次压缩必须买到 ≥ 迟滞带的真实 headroom，消除「压完即超标」空转。
  **scope 限定**：只约束 threshold auto 路径；manual force（c17）与 overflow Case1（c22）
  MUST NOT 受地板阈值约束——overflow 压缩是失败恢复手段，即使增益有限也优于原地 retry。
- **CompactionEnd 载荷扩展（pa-wire3 + c26 追加）**：成功 CompactionEnd 增加
  `tokens_after`（与 AfterCompaction settlement 同源同值）与 `notice`（一次性诊断文本，
  Option）。live-only，**不**持久化扩 CompactionEntry（c11 不动）；旧载荷解码缺省 None，
  resume/rebuild 落回 N 形词。事件序保持 end → settlement（先 quiet 计算，再发两端）。
- **压后大小呈现（app-tui-bridge compaction 块条款修订）**：块词形 `Compacted from N tokens`
  → `Compacted from N → M tokens`（保留展开和弦后缀）；`tokens_after` 缺省时落回 `N` 形。
  让压缩有效性一眼可验（本轮实测 35,840 → ~18k）。att29 不动（只登记折叠命中，不承载词形）。
- **一次性可行动诊断（新 domain 规则 c28 + bridge 呈现）**：auto（threshold / overflow）
  compact 成功且 AfterCompaction settlement ≥ contextWindow 时，经 CompactionEnd.notice
  发**每会话至多一次**的诊断（建议：调低 keepRecentTokens / 调高 contextWindow / 精简工具面）；
  TUI bridge 以滚动提示（ScrollNotice）一行尾插呈现（与 atc「失败与诊断 MAY 写滚动提示」同形），
  不改变 compaction 块折叠态。manual 路径不诊断。
- **lab 回放不变量 harness**（`#[ignore]`，人跑维护，不进 qa 门禁）：env 门控指向本地真实
  session JSONL（不脱敏入库），回放压缩管线 N 轮，断言：每次 compact 后投影单调下降、切点
  合法（c8）、摘要规模有界、追加固定量新内容后 compact 次数有界（无逐轮 churn）。

## Decisions（Open Questions 裁决记录）

1. **迟滞带取值**：固定比例常量（压后地板 × 25%），不加 `compaction.hysteresis` 配置——
   开箱即用纪律，c14 三字段不动；有真实需求证据再议配置化。
2. **no-shrink guard 不做独立 skip 分支**：地板 + 迟滞进阈值后，触发即保证
   `当前 tokens − 压后投影 ≥ 迟滞带`，结构性消除「压后 ≥ 当前 − margin」；保留的是
   **压后仍 ≥ window** 的一次性诊断，暴露真正坏掉的配置（固定开销吃满窗口）。
   摘要占位自适应（有上一次实测用实测）使地板估计在第二次 compact 起即收敛，最坏情形
   （首枚占位低估）至多多花一次 compact。
3. **词形 spec 落点**：`app-tui-bridge` compaction 块条款（词形 SSOT）+ `protocol-app`
   pa-wire3（载荷）+ `domain-compaction` c26 一句（同源值）；att29 只是折叠命中注册，不动。
4. **guard 与 overflow retry**：overflow Case1（含 Case1 已请求失败后的 compact-and-retry）
   不受地板阈值与诊断抑制——压缩是 overflow 的恢复手段，禁掉它用户只能干等失败。
5. **lab fixture 脱敏**：fixture 不入库——`#[ignore]` 测试经 env var 指向用户本地真实
   JSONL，测试文件 doc 给命令；仓库内零用户数据。

## Capabilities

- `domain-compaction`：c2 修订（地板阈值公式 + scope 限定）、c26 追加（tokens_after 同源）、
  新增 c28（一次性诊断条件与频度）；`reserve-trigger-shares-footer-estimate` 可执行场景
  比较式同步。
- `protocol-app`：pa-wire3 CompactionEnd 载荷 + 旧载荷解码纪律。
- `app-tui-bridge`：compaction 块词形 N → M + notice → ScrollNotice 尾插。
- `agent-runtime`：turn-end 触发义务条款的公式引用同步（义务本身不变）。
- 测试：`should_compact` / 阈值投影单测（32k 数值复现）、`tokens_after` / `notice` 单测、
  bridge 词形 fallback、`#[ignore]` lab 回放、BDD 新场景绑定。

## Impact

- 行为合约变更 → 完整 SDD pipeline（本提案，Specs landing 在绑定分支）。
- 涉及层：`agent/compaction`（阈值投影、orchestrator）、`agent/runtime/react`（once 旗标）、
  `protocol`（CompactionEnd 载荷 + wire 往返）、`app/core`（driver 投影）、`app/tui`
  （bridge 词形 + ScrollNotice）。
- 兼容红线：pa-wire 旧载荷形态 MUST 可解码为缺省（None）不 panic（c25 已立纪律，顺延）；
  会话格式与 `CompactionEntry` 形状不变（无 v6→v7 之类的格式变更）；大窗口常规配置
  （地板 + 迟滞 < window − reserve）行为零变化。
