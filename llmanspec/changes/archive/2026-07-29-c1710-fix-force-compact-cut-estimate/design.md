# Design: c1710-fix-force-compact-cut-estimate

## 根因（对照 pi）

| | pi | xylitol 现状 | 本 change |
|---|---|---|---|
| compact 输入 | `sessionManager.getBranch()` | `load_entries` → 整 JSONL | ✅ leaf 分支 |
| 切点累计 | `sessionEntryToContextMessages` + `estimateTokens(msg)` | `estimate_tokens_entry` JSON len/4 | ✅ 对齐 AgentMessage 估计 |
| 零贡献 | `messageTokens===0` → continue | 仍累加 0 / 杂讯 | ✅ skip |
| force 闸 | 仍 `prepareCompaction` | 同 | ✅ 保持；修尺子 |
| keep_recent=0 | 无 | 曾议 | ❌ 不做 |

footer Api ≫ 消息 chars/4 时：触发/展示跟 Api（c16），切点跟消息启发式（与 pi 同）——对齐后与 pi 同成败；误报来自 xylitol 低估。

## 落点

```text
XySessionStore 或 SessionManager
  └─ load_branch_entries(sid) ≈ get_branch(leaf)   // compact 专用入口

prepare_compaction / compact_session / Orchestrator::*
  └─ 一律 branch entries

find_cut_point
  └─ 每条 → as_agent_message / 等价 → estimate_tokens_message (pi 同构)
```

## 测试 seam

| Seam | 覆盖 |
|---|---|
| `find_cut_point` + 新估计 | thinking/toolCall 计入；JSON 低估样例在对齐后能切出 history |
| `prepare_compaction(branch)` | 消息体 > keepRecent → Ok；末条 compaction → Already compacted；真短 → Nothing to compact |
| Orchestrator force | 经 branch 路径；单测或 BDD `@req:c17` |
| 旁支不泄漏 | 文件序含 sibling、leaf 在右支 → compact 不计左支 |

## 非目标

force keep_recent=0 · 用 Api 直接当 keep 预算 · TUI 文案专案 · OTEL cancel（c1720）
