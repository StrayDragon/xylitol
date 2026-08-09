---
depends_on:
- c1880-update-responses-first-api-boundary
- c1890-add-responses-context-policy-assembler
branch: sdd/c1900-update-mcp-first-turn-tool-freeze
base_sha: 863d8836dd09dc4e7d3a9c77e6f7f59d465019f2
checkpointed: true
checkpoint_sha: c43e083e12a52e125d6799120fba02d982238686
---

# MCP 首条门闸 + 工具表定稿（移除 pending-turn 注入）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)
> **拆分（Q12）**：本 change **只**做门闸/定稿/去掉 next-turn 热并。`tool_search` 见活跃草案 [`c1960`](../../../delayed-changes/tools/c1960-add-tool-search-mcp-discovery/proposal.md)（双轨 B，后实现）。
> **工程约定**：code-first `defaults.rs`；本波不扩 YAML/env（超时等常量可进 defaults）。真源 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。
> **Lab**：Ornith/llama.cpp 上改 `tools[]`（upsert 或 append）均会 **cache miss**；故主路径 = **首轮前一次定稿后冻死**。

## Why

现状：MCP 异步 settle 后经 pending / next-turn 热并进 `ToolSet` → 后续请求 `tools[]` 变长或变描述 → 前缀缓存失效，且模型可能看到「中途多出来的工具」。

产品偏好（深挖 Q9–Q11）：**工具最好初始化后不变**。用户可打字提交首条，但 **生成卡住**，直到 system + 工具表（核心 + 当时已武装 MCP）就绪，再一次性定稿；之后本会话 **禁止** 再改顶栏 `tools[]`。

## What Changes

- **移除**「MCP pending → 下一 agent turn 才注入 tools」主路径。
- **首条生成门闸**：消息可进入队列/提交；ReAct/generate **等待** MCP settle（或 defaults 超时 / 失败策略）与 system prompt 装配完成后才开始。
- **定稿**：顶栏 `tools[]` = 内置核心 + 门闸通过时已武装 MCP；定稿后本会话冻结（`set_tools` / settle 不得再改 provider 可见表）。
- 改写与 ath23 等冲突的「未结算也可立刻跑 agent prompt」合约（Specs landing）。
- TUI/print：可键入；生成等待时有明确短 cue（文案钉 design）。
- 单测/BDD：门闸、超时、定稿后 settle 不再扩表。

## Capabilities（意向）

- `infra-mcp` / `agent-runtime` / `app-tui-host`（或等价）
- 产品：扩展能力-MCP、首轮就绪 UX

## Impact

- 首条可能多等 MCP；换来会话内 tools 前缀稳定（对 llama.cpp/Ornith 尤其重要）。
- 与旧「边聊边武装」行为不兼容 → 合约与文档明示。

## Out of scope

- `tool_search` / Deferred 发现 / BM25 / sidecar（→ `c1960`）
- 阻塞**键入**（只卡住生成）
- `/reload` 后是否二次门闸（待钉；默认倾向：reload 可触发一次重定稿并接受 bust）
- `c1920` epoch、状态栏、Anthropic Tool Search

## Parallel / depends

- **硬依赖**：`c1880`、`c1890`
- **软配合**：`c1930`（若定稿标记进 view）；`c1925` 可并行
- **后续**：`c1960` tool_search（不阻塞本 change）

## Open Questions

### 已钉（深挖，本 change 相关）

- **Q9**：工具表宜初始化后不变。
- **Q10**：去掉 pending-turn 注入；首条可提交、生成卡住等就绪。
- **Q11**：等 MCP settle（或超时）→ **Full 一次性定稿**；本会话不再改 `tools[]`。
- **Q12**：本 change 只做门闸/定稿；tool_search → **`c1960`**。
- **Q8**（仅作逃生笔记）：若未来仍被迫改表，方言用 **按 name upsert**，禁止盲追加同名；**不能**靠改表保 cache。
- **超时 / 部分失败（Q13）**：**超时后带已武装子集放行定稿**（未就绪 server 的工具本会话没有）。**不自动重试**连接；仅 **提示**用户可 `/reload` 再试。零 MCP 配置 → **立即放行**。任一条失败不阻挡已成功子集（与「子集放行」一致）；提示里汇总失败/超时诊断。

### 已钉 / 调研结论（续）

- **Resume × 稳定 tool id（Q15，[`调研稿`](../../../docs/research/responses-tools-stable-id-and-resume-mcp-2026.md)，[`tools id × resume MCP 调研`](bd5c389e-1cc9-4d2f-be35-6f21eb4cf256)）**：
  - OpenAI Responses **没有** function definition 级稳定 `id`，也 **没有** placeholder / slot / remap。
  - `call_id` / item `id` = **单次调用**身份，不能当工具定义别名。
  - 官方「稳前缀 + 动态发现」= `tool_search` + `defer_loading`（尾部注入）；**不能**无代价删改已加载集；Ornith lab **不可用** hosted 路径。
  - Codex resume = 重放 history + **当前 Config 重建 MCP runtime**；**不**做旧名→新名安全重映射。
  - **对本 change**：首轮定稿冻表仍只覆盖 **new session**；**resume 必须另做显式策略**（见 Q16）。**禁止**把「按 name 静默重绑且假装 cache 仍完美」写成安全恢复。
- **Resume 策略（Q16）**：**工具指纹一致 → 续冻；不一致 → 重定稿（按 name upsert 对齐当前 MCP，禁止盲追加同名）+ 用户可见 cue**，**接受 cache 降级**。原则：**正确性 / prompt 与可执行集一致 > 保 cache**。不能为了 cache 让 agent 看见过时或错位的 tools 表。
- **双轨产品（Q17）**：两条线都要能落地（不同 WirePolicy / compat）：
  1. **定稿/冻表轨（本 change / 方言默认，如 Ornith）**：首条门闸 + Full 定稿；resume/reload 按 Q16 upsert 重定稿；cache 可 miss。
  2. **`defer_loading` + `tool_search` 轨（`c1960`，声明支持的 provider）**：顶栏稳、发现走轨迹/output；日后用**已实现该语义的 model provider**做集成验证（非 Ornith 冒充）。两条线共享 registry / 门闸概念，**不以**假 hosted 混用。
- **`/reload`（Q14）**：**idle** 时再次门闸（同 Q13 超时/子集规则）→ **按 name upsert 重定稿** + 短 cue（工具表已刷新；接受 cache bust）；**busy 拒绝**（对齐现有 reload 闸）。**不**自动重试 MCP 连接（仅提示；用户可再 `/reload`）。

- **门闸提交 UI（Q18）**：**首条待跑与再次提交均用 follow-up 队列条视觉**（实现简单）。门闸结束后自动开跑首条；已入队的后续 follow-up 按既有 drain 语义。**不**在门闸期默认走 steer。
- **门闸 spinner（Q19）**：未提交 = **无** status lead spinner（idle；进度只走 welcome / 下轮预告 / `/mcp`）。已提交仍 GATING（含 idle `/reload` 后再提交）= status lead **`spinner + Assembling`**；定稿开跑后恢复既有 `Working` 等短词。队列条本身 **永不**带 spinner。禁止未提交时装假 `Working`。

### 待钉

- `c1960` 参考验证 provider 清单——实现时钉。

## Ethics

- risk_level: medium
- prohibited_actions: 定稿后静默再改 provider `tools[]`；用「禁用输入框」冒充门闸；把未就绪当成已定稿开跑
- required_evidence: 门闸单测/BDD；定稿后 settle 不再扩表；超时/失败路径有明确行为；ath23 等冲突合约已改
- refusal_contract: 不把「禁止键入」写成 MUST；不把 tool_search 写进本 change MUST
- escalation_policy: Q13/Q14/Q16/Q17/Q18/Q19 已钉；双轨验证 provider 升格 c1960 时另确认
