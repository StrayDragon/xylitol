---
change_id: c999-add-infra-provider-traffic-capture
title: "Provider 流量抓包 / 即时查看（外挂优先）"
status: purpose-draft
priority: 999
depends_on: []
author: agent
track: B
wave: observability-capture
---

# c999-add-infra-provider-traffic-capture

> **status: purpose-draft** — 仅提案意图与决策边界；升格 full（specs+tasks）前不 apply。
> **与 Track A 解耦**：hook 对齐见 `c735-update-agent-hooks-pi-parity`；本 change **不**把 SSE body 塞进 hook。

## Why

会话 JSONL / `XyChunk` 是**归一化后的结果**，无法单独证明「网关推了 `reasoning_text` 还是 `output_text`」。需要解耦于 agent 热路径的抓包与即时查看，用于调试通道错分、协议漂移、自建 `base_url`（如 `tufa`）。

## Purpose（草案）

1. **外挂优先**：抓包与 xylitol 进程解耦；默认不嵌入完整 MITM 栈。
2. **PoC 两条路径都必须被文档化验证**后再定产品形态：
   - **B1**：完整 [mitmproxy](https://mitmproxy.org/) 产品（底层可含 [mitmproxy_rs](https://github.com/mitmproxy/mitmproxy_rs)）经 `HTTP(S)_PROXY` / 本机 CA；
   - **B2**：适配 [claude-tap](https://github.com/liaohch3/claude-tap)（forward proxy 或 custom `base_url` reverse；可选 `--tap-client xylitol` / `--tap-store-stream-events`）。
3. **明确不选**：把 `mitmproxy_rs` **链进** xylitol crate（eBPF/进程重定向解决的是透明劫持任意进程，不是可控 `base_url` 的 LLM dump；HTTP UI 仍在 Python mitmproxy）。
4. 若 B1/B2 均不足（协议怪、要与 `XyChunk` 同屏对照）→ 后续另开 change 做薄 **dump proxy**（对齐 `../codex` `responses-api-proxy --dump-dir`），仍保持带外进程。
5. 可选诊断（可另 change）：进程内 `XYLITOL_DUMP_SSE` / `DUMP_CHUNK` structured log —— **补线级与映射级**，不替代外挂抓包。

## What Changes（升格后预期）

| 项 | 说明 |
|----|------|
| 调研结论落盘 | `docs/` 或本 change `design.md`：B1/B2 PoC 步骤、CA/代理与 reqwest 注意点 |
| 手测脚本/just | 可选 `just tap-mitm` / `just tap-claude` 文档命令（非强制进 `qa`） |
| 产品决策 | 选定「推荐默认外挂」；是否 PR claude-tap；是否立项 dump proxy |
| 合约（升格） | 可能新增 `infra-provider-capture` capability；MUST 含「默认零开销 / opt-in」 |

## Capabilities（升格时）

- 候选：`infra-provider-capture`（新建）或挂 `runtime-config`（代理/env 旋钮）

## Out of scope

- Track A hook 全量对齐（`c735`）
- 把原始成功 SSE 默认写入 session JSONL
- Helicone/Langfuse 云观测（可后置）
- TUI 内嵌完整抓包 UI（先外挂 viewer）

## Ethics

- risk_level: medium（日志含 prompt/密钥风险）
- prohibited_actions: 默认落盘明文 API key；默认开启抓包
- required_evidence: B1 与 B2 至少各一次本地 PoC 记录（成功或明确失败原因）
- escalation_policy: 外挂不足再开 dump-proxy change

## Depends

- **无硬依赖**（可与 `c735` 并行调研；实现互不阻塞）

## Decision log（草案）

| 选项 | 结论 |
|------|------|
| 集成 mitmproxy_rs 进主 crate | **否**（错层） |
| 使用完整 mitmproxy 作外挂 | **PoC 必做** |
| 适配 claude-tap | **PoC 必做**（对 coding-agent / 多 client 已成熟） |
| 自研 xylitol-tap | **仅当** B1/B2 不足 |

## Next

1. 升格前：完成 B1/B2 PoC 笔记 → 写入 `design.md`。
2. `llman-sdd-propose` 升格为 full（delta specs + tasks）。
3. 主线 hook 对齐继续走 `c735` 分波。
