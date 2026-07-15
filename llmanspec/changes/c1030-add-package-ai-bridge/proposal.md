---
change_id: c1030-add-package-ai-bridge
title: "新建 packages/xylitol-ai-bridge：LLM provider 接线 + 多源 token 计量"
status: full
priority: 1030
depends_on: []
author: agent
track: B
wave: provider-bridge
domain: packages
---

# c1030-add-package-ai-bridge

## Why

1. xylitol 需要稳定、可共享的 **client→LLM provider server** 接线面（方言 HTTP/SSE、归一化 usage/cost），而不仅是散落在 `src/infra/provider` 的实现。
2. Footer / compaction 需要**诚实、可标注来源**的上下文 token 数；chars/4 对中文/代码误差过大（见 c655 暂停结论与 tokenshub PRD 实验）。
3. 若只建独立 `tokenshub` 包，计量会与 provider 接线长期分叉；本 change 把 **接线 + 计量降级链** 收进同一包，避免架构偏移。

## Purpose

一步到位定义并落地（按 tasks 分阶段实施、一次归档）`packages/xylitol-ai-bridge`：

1. **Provider bridge**：迁入（或等价迁出后由 infra 薄映射）现有 LlmAdapter 族能力；包内使用**自有 DTO**，主 crate 映射到 `XyChunk` / `XyUsage` / `AgentMessage`（过渡双类型）。
2. **Accounting**：上下文计量优先级固定为
   `Api → RemoteCount → LocalTokenizer → Heuristic`（另保留 `Unknown` 语义供产品选用）；流路径禁止每 delta 全量 encode。
3. **下游只认内部语义**：经映射后的 `XyUsage` / `ContextTokenEstimate`（含 provenance）；agent/app 不解析 OpenAI/Anthropic 字段。
4. **集成**：compaction / session stats 改走 accounting；Driver 预留只读 Estimate API 形状（**不含** footer 产品文案；见依赖 draft **c1035**）。

## What Changes

- 新建 workspace 成员 `packages/xylitol-ai-bridge`
- 模块：`provider/` · `usage/` · `accounting/` · `tokenize/` · `registry/` · `fake/`
- `src/infra/provider` 变为依赖包 + DTO↔domain 映射（或阶段性 re-export）
- compaction / token 估计入口改用 accounting 优先级
- delta：`package-ai-bridge` · `package-ai-bridge-accounting` · modify `infra-provider` · modify `domain-compaction`

## Capabilities

- `package-ai-bridge`（新）
- `package-ai-bridge-accounting`（新）
- `infra-provider`（modify）
- `domain-compaction`（modify）

## Out of scope

- 产品 TUI footer `used N tokens` 文案与 harness → **c1035**（purpose-draft）
- 费用 ↑↓ / cache 分项 footer → **c1055**（depends_on c1035）
- 抽独立 `xylitol-llm-types` → **c1040**（本 change 用包内 DTO + 主 crate 映射）
- GGUF tokenizer → **c1045**；CLI prefetch/register → **c1050**；OpenAI RemoteCount 全量 → **c1060**
- 训练 tokenizer；默认静默大规模 HF 下载（ethics 禁止；保持 opt-in，不另开「默认静默下载」change）

## Ethics

- risk_level: medium
- prohibited_actions: 把 Heuristic 标成 Api；流上每个 TextDelta 全量 tokenizer.encode；未映射就让 agent 依赖包内方言 DTO；release 默认静默联网拉 tokenizer
- required_evidence: accounting 优先级单测；OpenAI/Anthropic usage 归一化对照；映射层不泄漏 vendor 类型出 infra/agent（对齐 pa7 精神）
- escalation_policy: 双类型映射成本过高或 arch_guard 冲突时升级用户确认是否改抽 types crate

## Depends

- []

## Downstream drafts（depends_on 本 change）

```text
c1030-add-package-ai-bridge
├── c1035-update-app-tui-footer-token-usage   # 吸收原 c655 调研
│   └── c1055-update-app-tui-footer-cost-breakdown
├── c1040-add-package-llm-types
├── c1045-add-ai-bridge-gguf-tokenize
├── c1050-add-cli-ai-bridge-tokenize
└── c1060-update-ai-bridge-openai-remote-count
```

- 原暂停调研备忘：`llmanspec/do-not-read-me/c655-update-app-tui-footer-context/`（由 **c1035** 吸收，勿平行复活两套 footer change）
