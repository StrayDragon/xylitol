---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
  - c1900-update-mcp-first-turn-tool-freeze
---

# tool_search + MCP Deferred 发现（另开；非开箱主路径）
> **一句话**：MCP 工具极多时 tools_mode=search 按需发现加载，靠 WirePolicy 声明分流（非开箱主路径）


> **⚠️ deferred（2026-09-08）**：仍在 `delayed-changes/`，未进活跃 graph。2026-08-10 排序 #17 作废。c1900 freeze 已归档，本票仍是非开箱 `tool_search` 轨，公开预览不挡。
> **对照**：`../codex` Deferred + `tool_search_*` + `defer_loading`。
> **Lab**：Ornith/llama.cpp **不支持** hosted `tool_search`（静默剥离）；`tool_search_*` item **400**；function 形可用。见下探测摘要。
> **调研**：`responses-tools-stable-id-and-resume-mcp-2026.md`（freeze） — **无** definition id/占位 remap；client `tool_search` 是 OpenAI 上追加能力的较好路线，**不能**单独解决 resume 删除/重命名；方言仍受 Ornith 限制。

## Why

当 MCP 工具极多、或不愿首轮灌全表、且 **WirePolicy 声明**端支持 `defer_loading` / `tool_search_*` 时，用内部 registry + 发现轨，尽量稳住顶栏前缀。

开箱个人 / Ornith 方言优先 `c1900`（freeze） 定稿冻表（正确性优先，cache 可降级）。**两条线都要实现**（Q17），靠声明分流，不靠运行时猜。

日后集成验证：接入**真正实现 defer_loading + tool_search（或 client execution）**的 model provider 做活测；禁止用 Ornith 冒充 hosted 通过。

## What Changes（意向，未实现）

- `tools_mode=search`：顶栏 = 核心 + 元工具；MCP Deferred。
- WirePolicy 双轨：hosted vs function 降级；**仅手动声明**，不自动探测。
- 后端默认内存 BM25（+ 可选显式 sidecar）；索引不落盘。
- 方言无 defer 时：按 name upsert（不得已）；知悉必 cache miss。

## Open Questions（Designed 已钉 · 见 design 决策表）

| ID | 钉死 |
|---|---|
| Q1 | Hosted vs ClientFunction 双轨；按声明选，不混假 hosted |
| Q2 | **仅手动声明**；禁运行时探测 |
| **Q-WP** | `WirePolicy.tool_search_wire`（独立字段）；**禁止**进 `ExtraPolicy`；与 `ToolsMode` 分层；见 design |
| Q3 | 检索 A+B，默认内存 BM25 |
| Q4 | sidecar **仅显式** |
| Q5 | 开箱 Search **作废**（默认 Full + c1900） |
| Q6 | description 刷新源 = registry；知悉 bust cache |
| Q7 | forge 严格按声明 |
| Q8 | 方言改表 → 按 name upsert |
| Q9–Q11 | 继承 c1900；Search 门闸冻顶栏非全 MCP 表 |
| Q14/Q16/Q17 | reload/resume/双轨：见 design；Ornith ≠ Hosted 证据 |
| Q-V | 假 provider 矩阵 Start/apply 足够；真网关清单 apply 前另钉 |

**工件**：[`design.md`](./design.md) · [`tasks.md`](./tasks.md)。**Start readiness**：Designed / **未** Branch binding。

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
