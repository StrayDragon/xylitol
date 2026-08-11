---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
---

# previous_response_id 可选链式续跑（配置开启）
> **一句话**：用 previous_response_id 只传增量续跑（配置开启）；兼容端声明支持才开，断链自动回退全量
> **当前排序**：#15（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#15**。


> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「Agent 如何调用大模型」多轮轨迹（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：默认仍全量重放；链式为 capability + 配置 opt-in。**断链**对齐 Codex：与上一请求比对 **非 input 字段**（model / instructions / tools / reasoning / …）；任一实质变化 → 回退全量 `response.create`（不引入独立 context epoch 计数）。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

Responses 支持用 `previous_response_id` 只传增量 input，可减重复传长轨迹。但兼容端参差、compact/换模/fork/工具表或 instructions 变化会断链。应先保证公共全量路径极致（`c1880`–`c1910`），再把链式当**可选优化**。全量重放路径不需要世代计数；链式才需要「上一请求非 input 快照是否仍可比」。

## What Changes

- 落盘并使用 `response_id`（今日常为 `None`）。
- capabilities + 配置打开时：连续 tool 环可发增量 + `previous_response_id`；`store` 策略按配置/文档（兼容端实测清单进 design）。
- **WirePolicy/compat 门闸**：仅当该 WirePolicy/compat 声明支持链式时允许开启；形似 Responses 但未验证的端点默认关，避免 DeepSeek/网关等静默丢上下文。
- **断链回退全量**：compact、换模、fork、**非 input 请求字段相对上一轮实质变化**（含 tools / instructions / model / reasoning 等）、配置关闭、缺 id、网关错误、compat/WirePolicy 不支持。对照姊妹仓 Codex `responses_request_properties_match` 直觉；本仓实现落点在 Assembler/会话续跑缝，不另发明 epoch 计数器。
- 观测：标明本轮 `full_replay` vs `chained` 与当前 WirePolicy/compat。
- 首版**不**自动探测是否支持。

## Capabilities（意向）

- `package-ai-bridge` / Responses adapter
- `agent-runtime`（何时可链）
- capabilities 配置（已归档 `c1880`）

## Impact

- 长 tool 环在官方 OpenAI 上可降重复传。
- 兼容端默认不受影响（关）。

## Out of scope

- 默认开启链式
- 自动发现网关能力
- 多模态专用 store 策略
- 独立 `context_epoch` / 前缀世代计数器（已否决；全量重放以当前 SSOT 为准）

## Parallel / depends

- **硬依赖**：`c1880`、`c1890`
- 建议不与同层改同一 adapter 文件的 change 抢冲突；或相关项稳定后再开本 change

## Further Notes（深挖 2026-08-10）

### 断链：非 input 快照比对（对照 Codex，映射本仓 body）

姊妹仓 Codex：`responses_request_properties_match` —— 比上一请求与本请求的 **非 input** 字段；`input` 另判「是否严格前缀延长」。本仓今日 Assembler 产出（`assemble_responses_body_with_diagnostics`）字段更瘦；拟钉比对面：

| 字段 | 变了是否断链 | 备注 |
|---|---|---|
| `model` | 是 | NextTurn 换模 |
| `tools` | 是 | 重定稿 / reload upsert / 指纹不一致 |
| `reasoning`（thinking 写入体） | 是 | thinking 档 / effort 变 |
| `include` | 是 | 如 `reasoning.encrypted_content` 有无 |
| `store` | 是 | 今日恒 `false`；若日后开 true 须进比对 |
| `prompt_cache_key` | 是（若 WirePolicy 允许写出） | 缺省双方皆无则相等 |
| `stream` | 是 | 同轮内通常不变；比上以免脏续 |
| `input` | 另规则 | **严格前缀延长**才可增量；否则全量 |
| `previous_response_id` | n/a | 续链目标，不参与「是否可比」等式左边 |
| `client_metadata` / 观测头 | 否 | 对齐 Codex：不影响续链可比性 |

**system / instructions**：本仓把 system **prepend 进 `input`**（无独立 `instructions` 顶栏）。故 `/reload` 改 AGENTS/skills → `input[0]` 变 → **前缀延长失败 → 断链**，不必另造 epoch。若日后拆出顶栏 `instructions`，须把该字段加入上表「是」。

**产品级断链触发（仍回全量，不依赖计数器）**：compact、fork、缺 `response_id`、网关错误、`extra_policy.previous_response_id=false`、compat/WirePolicy 不支持、上表任一非 input 不等、或 `input` 非严格前缀延长。

**否决**：独立 `context_epoch`（已删 `c1920`）。pi/Codex 均无此计数；Codex 用快照比对。

## Open Questions

### 已钉（深挖 2026-08-10）

- **`store` 默认**：产品默认 **`store: false`**（与今日 Assembler 硬编码一致）。**链式仍可试**：有上一轮 `response_id` 且非 input 快照可比 → 发增量 + `previous_response_id`；网关未持久 / 报错 / 找不到 id → **断链回全量**（禁止静默丢上下文）。**不**为链式默认改 `store: true`（隐私/上游磁盘门槛另议；若日后要开须产品确认，见 ethics escalation）。

### 仍开

- （暂无）propose/design 阶段若实测发现「`store:false` 下官方 OpenAI 链式不可用」，再开例外分支，不预先抬默认 store。

## Ethics

- risk_level: medium
- prohibited_actions: 断链后静默丢上下文；默认全员开链式
- required_evidence: 断链回退全量测；配置关时行为与今日一致
- refusal_contract: 不承诺一切兼容端支持 id 链
- escalation_policy: 开启 store 持久化上游侧须用户确认
