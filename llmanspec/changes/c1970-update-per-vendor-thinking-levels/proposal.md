---
depends_on:
- c1940-update-multi-api-named-compat
---

# 按厂商暴露 thinking levels（不再强推 xylitol STANDARD 梯子）

> **草案 only**：本 change **不得**在当前 c1940 分支上实现；待 `c1940-update-multi-api-named-compat`（多 api×compat）落地/归档后再 `ff` / `propose`。
> **产品方向**：用户看到的是**该厂商/该模型自己的档位**；xylitol 只存/配「厂商提供什么」；档位→wire 的映射留在 adapter/compat 本地，**不是**全局产品梯子的二次翻译。

## Why

今日 xylitol 有一套统一的 `ThinkingLevel` 梯子（`off` / `minimal` / `low` / `medium` / `high` / …，缺省时常落到 `ThinkingLevel::STANDARD`），再映射到各 provider 的 effort / budget / thinking 旋钮。用户先看到「我们的档」，再被静默映射成厂商语义——当厂商档位差很大时，**体感是错的**：看起来能选 `low`/`medium`，实际上游根本没有对应旋钮，或语义完全不同。

真实世界里：

- OpenAI 系 reasoning effort、Anthropic 官方 thinking budget、DeepSeek（Responses / Completions / Anthropic-compat）的 thinking 旋钮**不是同一把尺子**。
- 近端 DeepSeek flash v4 一类模型，用户侧往往只需要 **`off` / `high` / `max`** 三档；强推 STANDARD 五档只会制造虚假精度。
- 配置面已有 `thinking_levels`（及可选 `thinking_level_map`）作为**过渡**：可显式收窄某模型的可选档——这是 interim，不是最终产品心智。

本草案要研究并最终收口为：**按厂商（或按模型条目）表面向用户的档位集合**；产品层不再假装「全世界共用一把 STANDARD 梯子」。

## What Changes

验收向意向（正式 propose 时再落 specs；此处先钉可观察方向）：

- **用户面档位 = 模型/厂商声明的集合**：UI / slash / Driver 展示与 cycle 的档位，来自该模型配置（或 catalog/meta）给出的列表，而不是默认塞满 `ThinkingLevel::STANDARD`。
- **未配置 thinking 时**：行为须显式钉死（例如仅 `off`，或「thinking:false」不可调）；**禁止**再静默展开成与厂商无关的五档梯子作为默认产品语义。
- **映射本地化**：档位名 → 请求 body（effort / budget_tokens / `thinking` 对象等）的翻译留在 `api×compat` adapter；**禁止**在 agent/ReAct 层再维护「全局梯子 ↔ 全厂商」大表。
- **DeepSeek flash v4 近端**：配置/文档路径上可先用已有 `thinking_levels: [off, high, max]`（interim）；正式化后该列表应被视为「厂商面」而非「xylitol 标准子集」。
- **调研门禁（apply 前 MUST）**：对照并写清至少下列旋钮差异与可映射性——DeepSeek Responses、DeepSeek Chat Completions、DeepSeek Anthropic-compat、OpenAI（Chat Completions / Responses）reasoning、Anthropic Messages 官方 thinking；产出进 research 或 design，再改合约。
- **会话/持久化**：已写入的 thinking level 字符串若在新模型档位集外，须有明确 clamp / 拒绝 / 提示策略（propose 时钉一条），禁止静默改写用户意图而不告知。
- **非目标（本 change）**：不借机重做整套 WirePolicy；不在本分支改代码；不强制所有厂商档位名统一成同一枚举字面量（允许厂商字面量 + adapter 认领）。

## Capabilities（意向）

正式化时可能触及（名称待 propose 时确认，**不**在草案期建 specs）：

- `runtime-config` — `thinking_levels` / 映射字段的产品语义从「STANDARD 子集」改为「厂商面列表」
- `runtime-model-registry` — resolve / set / cycle 档位时以模型声明为准
- `package-ai-bridge` / `infra-provider` — adapter-local 映射；按 api×compat 分叉
- `protocol-model`（若仍保留跨面 Thinking 词汇）— 词汇表与「全局梯子」解耦或降级为可选标签
- 应用面（TUI/Print）— 展示与切换跟随当前模型档位集

## Impact

- **用户**：换模型时看到的 thinking 选项可能变短/变名，更贴近上游；减少「选了 medium 实际无效」的困惑。
- **配置**：现有显式 `thinking_levels` 成为正途而非权宜；缺省行为可能收紧（breaking 须在 propose 标明迁移）。
- **实现**：adapter 承担更多厂商差异；产品层更薄。依赖 `c1940` 的 api×compat 边界稳定后再动，避免与 Completions/Responses/Anthropic 三族接线打架。
- **对照**：不追平 pi 的全局统一档；xylitol 是个人 harness，优先「厂商诚实」。

## 依赖与时机

- **depends_on**：`c1940-update-multi-api-named-compat`（多协议族 × compat 定型后再动 thinking 面）。
- **本分支**：只允许存在本 `proposal.md` 草案；**禁止**附带实现、tasks、design、specs、attach。
- **下一步**：c1940 归档后，用 `llman-sdd-propose` / `ff` 正式化并补调研附件。

## Open Questions

- 跨厂商「同名不同义」（都叫 `high`）时，会话里只存字符串是否足够？是否要带 `(provider, api, compat)` 作用域？
- 枚举 `ThinkingLevel` 是保留为「常见标签超集」还是逐渐变成自由字符串 + adapter 校验？
- Anthropic budget（token 数）与 OpenAI effort（枚举）并存时，用户面要不要出现「高级：原始旋钮」逃生舱？

## Ethics

- risk_level: low（配置/展示语义；不涉及密钥或工具权限）
- prohibited_actions: 在本分支实现；伪造厂商不支持的档位；把全局 STANDARD 梯子写成 MUST 永久合约
- required_evidence: 正式 propose 前完成多厂商 thinking 旋钮对照调研
- refusal_contract: 不承诺所有兼容端点的 thinking 语义 ≡ 官方；未知 compat 不得瞎映射
- escalation_policy: 若要删除/重命名已落盘的 level 字符串或抬 session 版本，须单独显式确认
