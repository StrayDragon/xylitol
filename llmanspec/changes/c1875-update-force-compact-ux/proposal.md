---
depends_on:
  - c1860-refactor-context-token-settlement
---

# 手动 force compact UX（too-small 澄清 / 带 focus 重压）
> **一句话**：修 force compact 的 too-small 文案自相矛盾，澄清真实原因并支持带 focus 重压
> **当前排序**：#2（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#2**。


> **产品 A/B 确认前不 apply**。
> **观测分流** 不在本 change —— 见 [`c1870-add-otel-llm-infra-split`](../archive/2026-08-03-c1870-add-otel-llm-infra-split/proposal.md)。
> **依赖**：建议 c1860 归档后再推进（frontmatter `depends_on`）；与 c1870 无硬依赖，可并行 design，但失败 span 归属以 c1870 为准。

## 交接说明（给接手人）

用户痛点常被说成「阈值不够不能 compact」。实现上要拆成两层闸：

| 闸 | 谁走 | 含义 |
|---|---|---|
| **Reserve / 阈值**（footer `%`） | 仅 **auto** threshold | `tokens > window - reserve` 才触发；force **绕过** |
| **`prepare_compaction`** | force 与 auto 在真正压缩前 | 有无可摘要历史；英文失败串见下 |

可选 `/session-compact <instructions>`（**c1670**）只把文本追加进摘要 prompt（`Additional focus:`），**绝不**绕过 prepare。接手人 **禁止**实现「有 prompt 就跳过 prepare」除非产品明确选 B 且另写合约。

## Background（证据）

### Session 解剖（2026-08-03）

- **Id**：`460ad16e-874f-418f-8ca0-dabc58f89320`
- **文件**：`~/.xylitol/sessions/460ad16e-874f-418f-8ca0-dabc58f89320.jsonl`（~800KB）
- **形态**：~187 entries；**21×** `compaction`；tip 为普通 message（`hello` / 短回复），**不是** tip=Compaction 的 `Already compacted`
- **Footer**：约 `used 30k · 22.9%/131k` —— 30k 主要来自 keep 窗内已有 **compaction summary** + 少量新对话，不是「一大段从未压过的历史」
- **默认** `keep_recent_tokens`：20000（`src/agent/compaction/settings.rs`）
- 用户多次 `/session-compact` → ScrollNotice：`/session-compact failed: Nothing to compact (session too small)`（可叠多条）
- Langfuse 同次失败 = 独立 `agent.compaction` ERROR root（观测治理 → **c1870**，本 change 只关心面文案/块）

### 失败串与代码

| 用户可见（今日） | 源 | 典型原因 |
|---|---|---|
| `Nothing to compact (session too small)` | `prepare_compaction` | 空 leaf；cut 后 `history_count==0`；或 first_kept 无 id |
| `Already compacted` | 同上 | leaf **最后一条**已是 Compaction |
| （auto 静默） | orchestrator `maybe_*` | prepare 失败则不 compact，通常无 ERROR root |

关键文件：

- `src/agent/compaction/mod.rs` — `prepare_compaction` / `compact_session`
- `src/agent/compaction/orchestrator.rs` — `compact`（force：Start → prepare → …）
- `src/app/tui/effects/slash.rs` — `Err` → `push_scroll_notice("/session-compact failed: …")`；成功路径 `append_compaction_from_session`
- `src/agent/compaction/llm_summarizer.rs` — `with_additional_focus`（仅 prompt）
- 产品台账：`src/app/tui/PI_DELTAS.md` A05（c1670 已对齐可选 instructions）

### 与「阈值」的对照话术（可写进 notice）

> Force 不检查 context 占用百分比。当前失败是因为 **keep_recent 窗内已没有可再摘要的历史**（或 tip 已压缩）。Footer 的 % 只影响自动压缩。

## Why

- 文案 `session too small` 在已有 20+ 次 compaction、footer 显示 30k 时 **自相矛盾**，强化「阈值」误解。
- 用户直觉「带上 prompt 就能压」需要产品级回答：要么诚实拒绝并解释，要么提供 **显式重压 summary**（新能力），不能 silently 跳过 prepare。

## What Changes（意向）

### 文案（默认必做）

- ScrollNotice / 若保留 CompactionEnd.error：区分至少
  - `Already compacted`
  - 无可摘要历史 / 已在 `keep_recent` 窗（取代或补充裸 `session too small`）
- MUST NOT 暗示「再等阈值就会成功」。

### 产品分叉（propose 前必须问用户）

- **A（推荐默认）**：有无 instructions 都不过 prepare；只改进失败说明。instructions 仍只影响「真开压时」的 focus。
- **B（可选后续）**：显式「re-compact with focus」—— tip 附近已有 CompactionSummary 时，用 instructions **重写/再摘要上一份 summary**（新 req；成本 = 一次摘要 LLM）。**不是**「跳过 prepare」。

### TUI 块（design 可选）

- 今日 force 失败仍 `CompactionStart`→`CompactionEnd` + notice，易堆「失败块」。可改为 prepare 失败 **仅 notice**（须与 c1870「是否建 span」对齐，勿各改各的）。

### 观测

- 门闸失败进不进 Langfuse：**c1870**；本 change MUST NOT 偷渡 Collector / `xylitol.signal`。

## Capabilities（意向）

- `domain-compaction` — 失败分类文案；若选 B 则重压合约
- `app-tui-commands` / chrome — notice；可选减少失败 Compaction 块

## Impact

- 降低误解；若 B，用户可按主题主动重压（多一次 LLM 成本）。

## Out of scope

- 改 auto threshold / overflow 数值或触发公式
- OTel/Langfuse 分流实现（**c1870**）
- 恢复短名 `/compact`（A03）
- 无用户确认时实现 B

## Related history

| id | 关系 |
|---|---|
| c1640 / c1660 | force / overflow 路径 |
| c1670 | 可选 instructions → `Additional focus:` |
| c1680 / c1820 | TUI % / footer used tokens（易与本误解纠缠） |
| c1700 / c1870 | compaction 观测；失败归属 |
| c1860 | settlement；建议先归档 |

## Ethics

- risk_level: low（草案）
- prohibited_actions: 「有 prompt ⇒ 跳过 prepare」；auto 误用 manual instructions；未确认 B 就实现重压；偷渡 c1870
- required_evidence（propose）: 文案对照真实 leaf（建议仍用 `460ad16e-…`）；若 B 须一页与 pi/产品对齐的 design
- refusal_contract: 不把观测分流塞进本 change
- escalation_policy: A vs B **必须**用户确认后再 landing specs
