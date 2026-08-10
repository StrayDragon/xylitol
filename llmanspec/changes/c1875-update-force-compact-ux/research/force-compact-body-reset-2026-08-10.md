# Force `/session-compact` 与下一轮 provider body（代码事实）

> 一手来源：`src/agent/compaction/*`、`src/protocol/session/entries.rs`、`src/agent/llm_project.rs`、`src/app/tui/effects/slash.rs`；对照 `../pi/.../agent-session.ts` `compact()`。
> 日期：2026-08-10。

## 一句话

**今日**手动 `/session-compact`：**绕过** footer/% reserve 闸，但仍过 `prepare_compaction`；**只有 prepare 成功**才会打摘要 LLM、写 `CompactionEntry`，从而改变下一轮 provider body。prepare 早退 → **不调摘要 LLM、body 不变**。pi 同构（相同英文失败串）。

## 调用链（用户主动）

```text
/session-compact [instructions?]
  → Command::Compact { instructions }
  → XyDriver::compact → AgentSession::force_compact
  → CompactionOrchestrator::compact(reason=manual)
       emit CompactionStart
       load_leaf_branch
       prepare_compaction ──Err──► CompactionEnd(error) + Err
                                   slash: "/session-compact failed: {e}"
                                   ★ 无 LLM · 无新 CompactionEntry · 下轮 body 不变
       prepare Ok
       AgentCompactionSpan（otel19：仅此处起）
       compact_session → generate_summary(+ Additional focus?)
       append CompactionEntry
       CompactionEnd(ok) + settlement
       ★ 下轮 body 经 build_context_entries 被「重置」
```

`instructions` 只在 **真进入** `generate_summary` 时追加 `Additional focus:`；**绝不**绕过 prepare（`domain-compaction` c24）。

## 两层闸

| 闸 | Force | Auto (threshold) |
|---|---|---|
| Reserve（`tokens > window - reserve`；footer % 同源） | **绕过** | 必须过 |
| `prepare_compaction`（有无可摘要历史） | **必须过** | 失败则静默跳过 |

prepare 失败串（仍现行）：

- `Already compacted` — leaf **末条**已是 `Compaction`
- `Nothing to compact (session too small)` — 空 leaf / cut 后无可摘要消息 / first_kept 无 id

## 「重置下一轮 body」实际怎么做

磁盘 **不删**旧条目。下一轮读上下文时：

1. `load_leaf_branch` → 全 leaf 路径
2. `build_context_entries`：取**最新** `CompactionEntry`，输出 =
   `[最新 compaction] + [first_kept … compaction 前] + [compaction 后]`
   （`first_kept` 之前全部丢掉，不出 provider）
3. `as_agent_message`：compaction → `CompactionSummaryMessage`
4. `project_for_llm`：折成 user 行 `[Context summary: …]`
5. bridge 装配成 Chat/Responses body

### 样例 A — prepare 成功（body 真变）

Leaf（简化 id）：

```text
u_old, a_old, u_keep, a_keep, u_new, a_new
```

`keep_recent` 把切点落在 `u_keep`。compact 后追加 `c1`（summary=S，first_kept=`u_keep`）。

**下一轮 messages（逻辑）**：

```text
[Context summary: S]   ← 来自 c1
u_keep, a_keep, u_new, a_new
```

`u_old`/`a_old` 仍在 jsonl，但 **不进** provider body。

### 样例 B — 痛点会话（footer 仍有 30k，force 失败）

形态：多次 compaction 后，keep 窗内几乎只剩「上一份 summary + 短对话」，tip 不是 Compaction：

```text
… u_old… [已在更早 compaction 边界外]
c20 (summary=旧摘要), u_hi, a_hi   ← tip=普通消息
```

`find_cut_point(..., keep_recent=20k)` 可能把 `history_count` 算成 0 → prepare Err `session too small`。

结果：

- ScrollNotice：`/session-compact failed: Nothing to compact (session too small)`
- **无**摘要 LLM
- **无**新 `c21`
- 下一轮 body **仍是** `[Context summary: 旧摘要] + u_hi + a_hi`（未「重置」）

### 样例 C — tip 已是 Compaction

```text
…, c21 (最新 summary)
```

prepare → `Already compacted`。同样无 LLM、无新 entry、body 不变。
带 `/session-compact 聚焦 API` 也一样失败——instructions **到不了** `Additional focus:`。

## 与用户意图（已修订 · 2026-08-10）

曾探讨「主动必跑摘要 + 重置 body」——经对照 pi instructions 后 **否决**（误解 focus = 再压）。

**现行产品拍板（提案 A）**：保持 prepare 闸与 pi 同构；本 change **只**澄清失败文案。instructions 仍只在 prepare 成功后追加 `Additional focus:`。

空会话（零条目）仍无材料可摘要——合理硬边界。

## 相关合约指针

- `domain-compaction` c17（force 错误串 / 不过 reserve）、c24（instructions）、c9（迭代摘要）
- `infra-otel` otel19（prepare 早退不建 `agent.compaction`）— 已归档 c1870
- 架构心智：`docs/architecture/压缩与上下文.md`（偏 auto；未写 force 必跑 LLM）

## 压前 / 压后 body 对照（选型用）

记号：`S` = summary 文本；`[S]` = 进 provider 的 `[Context summary: S]` 行。

### 成功一刀（今日 xylitol ≈ pi）

| | 内容 |
|---|---|
| **压前 body** | `u1 a1 u2 a2 u3 a3 u4 a4`（假设很胖） |
| **切点** | `keepRecent≈20k` → 例如从 `u3` 起保留原文；`u1…a2` 进摘要 |
| **压后 body** | `[S_new]` · `u3 a3 u4 a4` |
| **盘上** | 旧消息仍在；多一条 `compaction(summary=S_new, first_kept=u3)` |

这就是选项 **b 心智**：摘要吃旧的，**最近原文留下**。

### 痛点：keep 窗已「吃满」（今日 / pi 都拒）

| | 内容 |
|---|---|
| **压前 body** | `[S0]` · `u_hi a_hi`（合计仍可能显示数万 token，因 S0 本身大） |
| **`/session-compact`** | prepare → `session too small`（没有「比 keep 更旧、还可再切」的原文） |
| **压后 body** | **不变**（无新 entry） |

若强制再压，两种产品结果：

| 选型 | 压后 body | 直觉 |
|---|---|---|
| **a 几乎只留摘要** | `[S1]`（`u_hi a_hi` 也进摘要，keep≈空） | 最瘦；细节只靠 S1 |
| **b 仍留一小段原文** | `[S1]` · `u_hi`（或最近 N token） | 接近今日成功路径的 keep 心智 |

### tip 已是 Compaction（今日 / pi：`Already compacted`）

| | 内容 |
|---|---|
| **压前 body** | `[S0]`（末条就是 compaction，后面无新消息） |
| **今日 / pi** | 拒绝；body 不变 |
| **若 a** | `[S1]`（重写/再摘要 S0，± focus） |
| **若 b** | 与 a 几乎相同（没有「最近原文」可留） |

## `../pi` 怎么设计的（一手）

来源：`pi/packages/coding-agent/docs/compaction.md`、`docs/session-format.md`、`src/core/compaction/compaction.ts` `prepareCompaction`、`src/core/agent-session.ts` `compact()`。

| 点 | pi 行为 |
|---|---|
| 手动 `/compact [prompt]` | 有；instructions = 摘要 focus（对齐 xylitol `/session-compact`） |
| 是否过 reserve/% | **手动不过**阈值；`enabled:false` 仍可手动 |
| 是否过 prepare | **过**——与 auto **同一** `prepareCompaction` |
| tip=compaction | `Already compacted`，**不**跑摘要 |
| 无可切旧史 | `Nothing to compact (session too small)`，**不**跑摘要 |
| 成功后 keep | **始终**按 `keepRecentTokens`（默认 **20000**）留最近原文 → 产品上就是 **b** |
| 成功后 runtime | `appendCompaction` → `buildSessionContext()` → `agent.state.messages = …`（下轮 body 重置） |
| 新格式 | 可选 `retainedTail`（摘要条目上直接挂保留消息）；旧格式仍 `firstKeptEntryId` |
| 扩展 | `session_before_compact` 可 cancel 或自带 summary；**不**改变默认 prepare 闸 |

结论：**pi 没有「主动就一定跑摘要」**；主动只跳过阈值，仍可因 prepare 空手返回。成功时的 body 形状固定是「新 summary + ~20k 原文 keep」，不是「只剩 summary」。
