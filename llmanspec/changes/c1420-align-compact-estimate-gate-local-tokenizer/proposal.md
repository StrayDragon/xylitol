---
change_id: c1420-align-compact-estimate-gate-local-tokenizer
title: Compact 与 TUI 同源计量；LocalTokenizer 仅 on/off 且默认 off
status: purpose-draft
priority: 1420
depends_on: []
author: agent
track: B
wave: token-estimate-align
domain: agent
apply_band: P0
branch: feat/c1420-align-compact-estimate-gate-local-tokenizer
---

# c1420-align-compact-estimate-gate-local-tokenizer

> **purpose-draft（先钉意图，不急 apply）**
> 明确不做：跨进程 IPC / Single-flight 丢弃旧结果等复杂防抖。Footer 已有 host 侧 async + generation 即可。

## Why

1. **计量分叉**：产品 TUI footer 走 `Driver::estimate_context_tokens` → paa1（Api → RemoteCount → LocalTokenizer → Heuristic）；`CompactionOrchestrator::maybe_auto_compact` 仍用 `message.to_string().len()/4`。用户看到的「used N tokens」与真正触发 compact 的阈值**不是同一尺子**，阈值/安全感都难对齐。
2. **LocalTokenizer 默认太贵**：只要 registry 能解析词表（含已 download 的 HF），当前 estimate 就会在 LocalTokenizer 档跑 encode。Footer 每轮刷新可卡 ~1s；用户多数时间有 Api usage（本机 openai-compat 已验证），本地 encode 应是**显式打开**的精确档，不是默认路径。
3. **策略先砍窄**：every-N / idle / single-flight 排队防打满 CPU 等先不做；只留 **`on` | `off`**，**默认 `off`**，关时落到 Api /（可选 RemoteCount）/ Heuristic。

## 实测（当前配置 `qwen` → `http://tufa:50256/v1`）

对 llama.cpp 兼容端点实打：

| 探测 | 结果 |
|---|---|
| `POST /v1/chat/completions`（非流） | 有 OpenAI 形 `usage`：`prompt_tokens` / `completion_tokens` / `total_tokens`；另有 `prompt_tokens_details.cached_tokens`、`timings` |
| 流式 + `stream_options.include_usage` | 末包带同形 `usage` |
| `POST /v1/responses/input_tokens` | 返回 `{"input_tokens":…,"object":"response.input_tokens"}` |

结论：**Api 锚点对本机端可用**（bridge Completions 已映射 `prompt_tokens→input`）。Compact 对齐 footer 后，有 usage 时应优先跟 Api，而不是继续 len/4；LocalTokenizer 默认关也不妨碍「有 usage 就准」。

## Purpose

1. **Compact 触发阈值**与 **TUI footer 展示**共用同一套估计入口 / 优先级（paa1），含同一 Api 锚点规则（paa2）与 provenance。
2. **LocalTokenizer 闸**：配置仅 `on` | `off`（命名待 tasks 钉死，如 `token_estimate.local_tokenizer: off`），**默认 `off`**；`off` 时估计路径 MUST NOT 调用本地 encode（含 builtin / HF cache / local path）；`on` 时才进入 paa1 的 LocalTokenizer 档。
3. **明确非目标**：Single-flight + 丢弃旧结果跨进程设计；every-N / idle 节流策略；TUI 下载确认；改 compact 摘要算法本身。

## What Changes（规划）

- `maybe_auto_compact`（及必要的 cut/阈值旁路若仍硬编码 heuristic）改为经 `estimate_context_tokens_with`（或 Driver/orchestrator 共享 helper），注入与 footer 同级的 model_id / tokenizer_override / local 闸。
- `EstimateOpts`（或 AppConfig）增加 `allow_local_tokenizer: bool`（默认 false）；registry 有映射也不自动 encode。
- **先修**：ReAct 持久化 `XyChunk::Done` 的 `usage` / `stop_reason`（本 change 内，不拆号）。
- Specs：`agent-runtime`（usage 落盘）+ `domain-compaction`（阈值同源）+ `package-ai-bridge-accounting`（local 闸）+ `runtime-config`（YAML `on|off`）。
- BDD：Done 带 usage → session 可 Api；compact 阈值同源；`local=off` 不得 LocalTokenizer。

## Capabilities

- `agent-runtime`（modify）
- `domain-compaction`（modify）
- `package-ai-bridge-accounting`（modify）
- `runtime-config`（modify）

## Impact

- 默认行为：footer / compact 更常走 Api 或 Heuristic，**少算 HF encode** → 更轻。
- 开 `local: on` + 已 download 词表 → 与今日「有映射就 encode」接近，供本地无 usage / 要更准的场景。
- Compact 触发时机相对今日 len/4 **可能变早/变晚**（尤其中文）；应用 Api 后通常更贴近真实窗口。

## Out of scope

- Single-flight / IPC / 跨进程排队
- RemoteCount 默认策略变更（仍保持显式 enable）
- ~~流式 include_usage~~：bridge Completions **已开** `include_usage`；根因是 **ReAct 落盘丢弃** `XyChunk::Done.usage`（见下「发现」）——宜单独小 change 或并入本 change 前置任务

## 发现（查选项 4，2026-07-20）

端点有 usage，但会话**从未落盘**：

1. Bridge：`openai.rs` 流末包解析 `usage` → `AiBridgeChunk::Done { usage }`；`infra/provider/map` 映射到 `XyChunk::Done { usage }`。
2. **断点**：`src/agent/runtime/react.rs` 对 `XyChunk::Done { .. }` 空处理；持久化 `AssistantMessage` 硬编码 `usage: None` / `stop_reason: None`（`partial_assistant_message` 同）。
3. 抽查 `~/.xylitol/sessions/*.jsonl` 最近多条：assistant 条目**无** `usage` 字段。

→ Footer 估计拿不到 Api 锚点，只能 LocalTokenizer（若映射+缓存）或 Heuristic——这解释了 HF encode 卡顿与「接口明明有 token 数却用不上」。
