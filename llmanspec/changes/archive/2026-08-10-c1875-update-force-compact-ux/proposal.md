---
depends_on:
- c1860-refactor-context-token-settlement
branch: sdd/c1875-update-force-compact-ux
base_sha: 1ed6c0348dbca72c8c2e1058f69a2eef4fadffdc
checkpointed: true
checkpoint_sha: 1ed6c0348dbca72c8c2e1058f69a2eef4fadffdc
---

# 手动 force compact UX（too-small 澄清）
> **一句话**：修 force compact 的 too-small / Already compacted 文案自相矛盾，澄清真实原因；**不**改变 prepare 闸与 pi 对齐的 instructions 行为
> **当前排序**：active（2026-08-10 升格；排序软标签）
> **调研**：[`research/force-compact-body-reset-2026-08-10.md`](./research/force-compact-body-reset-2026-08-10.md)


> **依赖**：`depends_on` c1860 **已归档**（2026-08-03）；闸已满足。观测分流 **c1870 已归档**（prepare 早退不再导出 `agent.compaction`）；本 change MUST NOT 偷渡 Collector。
> **产品拍板（2026-08-10）**：选 **A — 仅文案**。不强制再压；instructions 保持与 pi 同构（见下）。曾探讨的「主动必跑摘要」已否决（误解 focus = 再压）。

## 交接说明（给接手人）

用户痛点常被说成「阈值不够不能 compact」。实现上要拆成两层闸：

| 闸 | 谁走 | 含义 |
|---|---|---|
| **Reserve / 阈值**（footer `%`） | 仅 **auto** threshold | `tokens > window - reserve` 才触发；force **绕过** |
| **`prepare_compaction`** | force 与 auto 在真正压缩前 | 有无可摘要历史；英文失败串见下 |

### instructions（与 pi 同构 · 本 change 不改行为）

`/session-compact [<instructions>]`（pi：`/compact [instructions]`）：

1. **先** prepare；失败 → instructions **用不上**（与无参相同错误）。
2. 成功后非空 instructions 在结构化摘要 prompt **追加** `Additional focus: …`（不替换 Goal/Constraints 骨架）。
3. split-turn：只进 **history** 摘要；turn-prefix **不**注入。
4. auto **不传** instructions。
5. Focus **只影响摘要写什么**；不改切点 / `keep_recent` / 是否过 prepare。

接手人 **禁止**实现「有 prompt 就跳过 prepare」或「主动必跑摘要」——已明确否决。

## Background（证据）

### Session 解剖（2026-08-03）

- **Id**：`460ad16e-874f-418f-8ca0-dabc58f89320`
- **形态**：多次 compaction；tip 为短对话；footer ~30k 主要来自 keep 窗内已有 summary
- 多次 `/session-compact` → ScrollNotice：`/session-compact failed: Nothing to compact (session too small)`（可叠多条）

### 失败串与代码（仍现行）

| 用户可见（今日） | 源 | 典型原因 |
|---|---|---|
| `Nothing to compact (session too small)` | `prepare_compaction` | 空 leaf；cut 后 `history_count==0`；或 first_kept 无 id |
| `Already compacted` | 同上 | leaf **最后一条**已是 Compaction |
| （auto 静默） | orchestrator `maybe_*` | prepare 失败则不 compact |

关键文件：`compaction/mod.rs`、`orchestrator.rs`、`llm_summarizer.rs`（`with_additional_focus`）、`tui/effects/slash.rs`、`protocol/session/entries.rs`（`build_context_entries`）。

### 与「阈值」的对照话术（可写进 notice）

> Force 不检查 context 占用百分比。当前失败是因为 **keep_recent 窗内已没有可再摘要的历史**（或 tip 已压缩）。Footer 的 % 只影响自动压缩。带 instructions 也不会绕过该闸——focus 只在真开压时影响摘要内容。

### pi 对照（instructions）

与 xylitol **行为对齐**（详见调研文）：prepare 同闸；`Additional focus:` 追加；不替换骨架；turn-prefix 不注入。pi **也没有**「主动必跑 / 强制再压」。

## Why

- 文案 `session too small` 在已有多次 compaction、footer 显示数万 token 时 **自相矛盾**，强化「阈值」误解。
- 根因是 prepare（无可切旧史 / tip 已 compaction），不是阈值；修文案即可对齐心智，**不必**偏离 pi 去强制再压。

## What Changes

### 文案（默认必做 · 产品 A）

- ScrollNotice / 若保留 CompactionEnd.error：区分至少
  - `Already compacted`（**保留**；2026-08-10 拍板不改白话）
  - **替换**裸 `session too small`（替换而非补充）；最终用词见 `design.md`
- MUST NOT 暗示「再等阈值就会成功」。
- MUST NOT 暗示「加上 instructions 就能压」。

### 明确不做

- **不**强制再压 / 不跳过 prepare（否决旧探讨 B 与「主动必跑」）。
- **不**改 instructions 注入语义（保持 c24 / pi）。
- **不**改 auto threshold / keep_recent 公式。

### TUI 块（design 可选 · 非本波默认）

- 今日 force 失败仍 `CompactionStart`→`CompactionEnd` + notice。可另议改为 prepare 失败仅 notice；不阻塞文案切片。

### 观测

- 门闸失败进不进 Langfuse：**c1870 已归档**；本 change MUST NOT 偷渡 Collector / `xylitol.signal`。

## Capabilities

- `domain-compaction` — 失败分类文案（若合约需从裸英文串升级为更诚实措辞）
- `app-tui-commands` / chrome — notice 文案

## Impact

- 降低「阈值 / 带 prompt 就能压」误解；行为与 pi 保持一致。

## Out of scope

- 改 auto threshold / overflow 数值或触发公式
- 强制再压 / 跳过 prepare
- OTel/Langfuse 分流实现（**c1870**）
- 恢复短名 `/compact`（A03）
- 改 `Additional focus:` 语义或引入 replaceInstructions

## Related history

| id | 关系 |
|---|---|
| c1640 / c1660 | force / overflow 路径 |
| c1670 | 可选 instructions → `Additional focus:` |
| c1710 | cut 计量；too-small 误报曾修过一类 |
| c1680 / c1820 | TUI % / footer（易与误解纠缠） |
| c1700 / c1870 | compaction 观测（已归档） |
| c1860 | settlement（已归档；depends 满足） |

## Open Questions（propose 前）

1. ~~失败英文串替换 vs 补充~~ → **已决：替换**裸 `session too small`（非补充叠句）。
2. ~~`Already compacted` 白话~~ → **已决（2026-08-10）：保留**现状英文。
3. ~~新 too-small 用词~~ → **已决于 design**：见 `design.md`「失败串 SSOT」。

## Ethics

- risk_level: low
- prohibited_actions: 「有 prompt ⇒ 跳过 prepare」；主动必跑摘要 / 强制再压；auto 误用 manual instructions；偷渡 c1870；恢复 `/compact` 短名；保留误导性 `session too small` 作为用户可见主串
- required_evidence（propose）: 文案对照真实失败原因（建议仍用 `460ad16e-…` 心智）；与 pi instructions 对照表（调研文）；design 失败串表
- refusal_contract: 不把观测分流或强制再压塞进本 change
- escalation_policy: Open Q 已收口；Specs landing 后进 apply
