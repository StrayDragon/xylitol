---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
  - c1900-update-mcp-first-turn-tool-freeze
---

# tool_search + MCP Deferred 发现（另开；非开箱主路径）

> **活跃草案（实现后置于 `c1900`）**：与 [`c1900`](../c1900-update-mcp-first-turn-tool-freeze/proposal.md) **拆分（Q12/Q17 双轨）**。开箱主路径 = 门闸 + Full 定稿；本 change = `defer_loading`/`tool_search` 轨（WirePolicy 声明后）。
> **对照**：`../codex` Deferred + `tool_search_*` + `defer_loading`。
> **Lab**：Ornith/llama.cpp **不支持** hosted `tool_search`（静默剥离）；`tool_search_*` item **400**；function 形可用。见下探测摘要。
> **调研**：[`responses-tools-stable-id-and-resume-mcp-2026.md`](../../../docs/research/responses-tools-stable-id-and-resume-mcp-2026.md) — **无** definition id/占位 remap；client `tool_search` 是 OpenAI 上追加能力的较好路线，**不能**单独解决 resume 删除/重命名；方言仍受 Ornith 限制。

## Why

当 MCP 工具极多、或不愿首轮灌全表、且 **WirePolicy 声明**端支持 `defer_loading` / `tool_search_*` 时，用内部 registry + 发现轨，尽量稳住顶栏前缀。

开箱个人 / Ornith 方言优先 [`c1900`](../c1900-update-mcp-first-turn-tool-freeze/proposal.md) 定稿冻表（正确性优先，cache 可降级）。**两条线都要实现**（Q17），靠声明分流，不靠运行时猜。

日后集成验证：接入**真正实现 defer_loading + tool_search（或 client execution）**的 model provider 做活测；禁止用 Ornith 冒充 hosted 通过。

## What Changes（意向，未实现）

- `tools_mode=search`：顶栏 = 核心 + 元工具；MCP Deferred。
- WirePolicy 双轨：hosted vs function 降级；**仅手动声明**，不自动探测。
- 后端默认内存 BM25（+ 可选显式 sidecar）；索引不落盘。
- 方言无 defer 时：按 name upsert（不得已）；知悉必 cache miss。

## Open Questions（已钉，升格时继承）

- Q1 wire 双轨；Q2 仅手动声明；Q3 A+B 默认 A 内存；Q4 sidecar 仅显式；Q5 开箱 Search **作废**（主路径在 c1900）；Q6 description 刷新 source（知悉 bust cache）；Q7 forge 吃得下的形状；Q8 upsert；Q9–Q11 见 c1900。

### Lab（Ornith）

| 探针 | 结果 |
|---|---|
| `type: tool_search` | 200 但静默剥离 |
| `function` name `tool_search` | 接受 |
| `tool_search_*` input item | 400 |
| `defer_loading` | 忽略 |

## Out of scope

- 首条门闸 / 移除 pending-turn（→ `c1900`）

## Ethics

- risk_level: medium
- prohibited_actions: 未声明当 hosted；盲追加同名 tools
- required_evidence: 升格时假 provider + 声明表；Ornith 不标 hosted
- refusal_contract: 不把本 change 写成开箱 MUST
- escalation_policy: 与 c1900 定稿策略冲突时以产品确认档位为准
