---
depends_on:
  - c1890-add-responses-context-policy-assembler
---

# Session SSOT ↔ Provider view：有序幂等转换契约

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（术语对照 §7）；落点备忘 [`landing.tmp.md`](./landing.tmp.md)
> **书指针**：《深入理解 AI Agent》Ch2 轨迹 vs 投影；书语仅经 research §7 映射，**禁止**写入 live specs。
> **自包含**：钉 **protocol Session JSONL ↔ agent 投影 ↔ xylitol-ai-bridge Assembler** 的互转边界——**顺序稳定、可重复（幂等）**，服务 Prompt Cache / KV 前缀；不实现状态栏 / search / 压缩算法。
> **工程约定（本波次）**：策略默认 **code-first**；**不**新增 YAML/env 旋钮。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

主仓同时持有：

- **Session SSOT**：`SessionEntry` JSONL（`SESSION_VERSION`）+ `AgentMessage`（Llm∪Env）
- **Provider view**：`AiBridgeMessage` → Responses `input`（`ResponsesAssembler`，包 `xylitol-ai-bridge`）

用户 **resume** / **import session** 后，最终打到 API 的消息前缀若与「同进程续跑」不一致，Prompt Cache / KV 会无故失效（比状态栏问题更常见、更可测）。

需要一层薄而硬的 **转换 + 前缀幂等合约**（字段级：system / tools / input 历史序与内容），并用 **同库 lab + 默认 live-provider 模型 + Langfuse**（`observation.input` / `cache_read`）做证据——而不是再造 Codex `ResponseItem` 史，也不是本波做状态栏。

## What Changes

- **规范性**：Session SSOT vs Provider view；resume/import 与同进程续跑的前缀对齐目标；转换矩阵（无损/有损）；顺序不变量。
- **路径唯一**：`load`/`import` → `build_context_entries` → `as_agent_message` → `project_for_llm` → `ResponsesAssembler`；禁止 infra 平行 Env 折叠。
- **可测**：
  - 离线：JSONL/fixture → assemble `input`（+tools）规范化哈希相等（跑两次 / resume 形）。
  - 在线（维护 lab，不进 qa）：`live-provider.local.yaml` + 可选 Langfuse；resume 臂 `cache_read` 不低于同条件续跑。
- **指针**：system date 日界 → `c1905`；tools 冻表 → `c1900`；状态栏 → `c1895`（本波不做）。
- 被引用方：后续 harness-meta / 冻结表实现时 MUST 遵守本前缀纪律。

## Capabilities（意向）

- `agent-session`（加载 / 投影边界）
- `package-ai-bridge`（Assembler `input` 顺序与幂等；衔接 pab15/17/24/25）
- 必要时一句 `domain-compaction` / `agent-runtime` 交叉引用（不扩产品）

## Impact

- resume / 多轮：同盘面可预期重建 Responses 前缀。
- 下游 harness-meta / 冻结表有共同「存哪 / 怎么折」语言，而不绑架本波实现栏。

## Out of scope

- **Agent 状态栏** UI / 读数 / `AgentStatusBar` 落盘（→ delayed `c1895`）
- tool_search 实现（→ `c1960`）
- 压缩算法与冻结表实现（→ delayed `c1910`；本波最多文档指针）
- Todo 产品（→ delayed `c1955`）
- Session SSOT 改为 Codex `ResponseItem` 数组
- `previous_response_id` 链式（→ delayed `c1915`）
- 导出 strip 产品旋钮（无本波新 kind 则无强制实现）
- 新插件式 meta 市场

## Parallel / depends

- **硬依赖**：`c1890`（已归档）
- **已衔接**：`c1925`（已归档）——thinking 全量回放与同轮顺序
- **下游**：delayed `c1895`、`c1910`；`c1960`；Todo `c1955`
- `c1920` / `c1935` delayed

## Decisions

### Explore 2026-08-05（仍有效的指针；本波不落地产品）

1. 未来 harness-meta：**独立** kind（非裸 `customMessage`/user）；细节 → `c1895`。
2. 旧错盘：版本≠`SESSION_VERSION` **拒绝**（已有）；不做静默 migrate。
3. 导出 strip / HTML strip：留给有独立 kind 之后。
4. 冻结表：JSONL entry/header → `c1910`。

### 收窄 2026-08-06（本波主钉）

5. **主目标** = resume/import 后 API **消息前缀**与同进程续跑尽可能一致（为 cache），**不是**状态栏。
6. **幂等**：同 leaf + 同 `(system_prompt, tools, thinking_level, WirePolicy)` → assemble 的 `input`（及约定的 tools/reasoning 闸）规范化相等。
7. **顺序**：叶序保留；同轮 `reasoning` → text → `function_call`（c1925）；system/developer 前置。
8. **有损折叠**文案必须形状稳定；改文案 = 破坏前缀，须显式 change。
9. **证据**：离线哈希单测 + 维护 lab（live-provider 默认模型）；Langfuse 观测 `observation.input` / `cache_read`（不进 qa）。
10. **禁止** infra 第二套 Env 折叠；禁止展示 thinking 冒充 signature。

## Open Questions

- （主线已收窄；specs landing 时复核 s20「未知 type 跳行」与「版本硬拒」边界是否写进本 change 还是保持既有。）

## Ethics

- risk_level: medium（持久化与请求前缀）
- prohibited_actions: infra 平行折叠；无故改稳定折叠文案；把 SSOT 改成 ResponseItem 史；本波实现状态栏冒充进度
- required_evidence: 转换矩阵入 design；幂等 / 顺序 golden 或单测；与 c1925 顺序一致
- refusal_contract: 不承诺一切 Env 投影可逆；不承诺兼容端 cache 语义 ≡ 官方
- escalation_policy: 改折叠文案或抬 `SESSION_VERSION` / 新增盘面 kind 须显式 change
