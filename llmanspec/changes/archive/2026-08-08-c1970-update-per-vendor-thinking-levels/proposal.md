---
depends_on:
- c1940-update-multi-api-named-compat
branch: sdd/c1970-update-per-vendor-thinking-levels
base_sha: 77d4be5d7dbaed6e529681363c828c5b0743bdc5
checkpointed: true
checkpoint_sha: 77d4be5d7dbaed6e529681363c828c5b0743bdc5
---

# 按厂商暴露 thinking levels（配置声明，取消 STANDARD 默认）

## Why

今日在 `thinking: true` 且未声明列表时，静默展开 `ThinkingLevel::STANDARD`（off…high），再经封闭枚举校验配置。用户看到的是「我们的梯子」，不是厂商真实旋钮；DeepSeek 等近端模型档位短且映射非线性，虚假精细度会误导。

`c1940` 已稳定 api×compat；本 change 把用户面档位收口为 **配置声明的字符串列表**，映射留在请求边界，并保证 **会话 resume / 推理重放无损**。

## What Changes

- **支持集 = 配置声明**：`thinking_levels` 为用户面 SSOT；允许厂商字面量（不要求属于历史封闭超集）。`thinking: false` 或未声明可调列表 → **仅 `off` / 不可调**（breaking：取消 STANDARD 静默默认）。
- **换模默认**：可调模型取配置列表 **末项**；不可调为 `off`。Settings.default 仅首次装配且须 ∈ 支持集，不得覆盖换模末项策略。
- **映射本地化**：档名 → body 经 `thinking_level_map` / api×compat adapter；agent/ReAct 不维护全球梯子大表。
- **resume / 重放无损**：
  - 会话 `thinkingLevelChange` 存精确字符串；load 还原末次字符串，**禁止**因集外而改写 JSONL 或静默追加 clamp 条目。
  - 配置漂移允许 sticky out-of-set 直至用户显式改档。
  - Responses 等推理 signature **全量回放**策略不变（与档位选择正交）。
- **产品 UI**：展示/ cycle 跟随当前模型声明列表（不再写死「只暴露 xylitol 枚举名」）。
- **非目标**：不重做 WirePolicy；不在本 change 接入 models.dev（档位原料后置 `c1980`）；不把 Anthropic 原始 budget 滑条做成必选 UI。

## Capabilities

- `runtime-config` — `thinking_levels` / `thinking_level_map` 语义
- `runtime-model-registry` — resolve / set / cycle / 换模默认 / resume 还原
- `package-ai-bridge` — 字符串档 → api×compat 请求体（单测/既有组装缝）
- `app-tui-commands`（及必要 chrome）— `/model` 槽展示声明档
- `agent-session` / store — resume 还原与落盘不丢字

## Impact

- **Breaking**：未写 `thinking_levels` 的可 thinking 模型不再出现五档；需显式声明或保持不可调。
- **会话**：旧 JSONL 中已有枚举字面量仍可还原；新档名只要配置声明即可。
- **实现**：弱化/退役产品面封闭 `ThinkingLevel` 超集义务；运行时以字符串为 SSOT（迁移细节见 design，不钉类型名进 MUST）。

## Ethics

- risk_level: low
- prohibited_actions: 伪造厂商不支持的档；resume 静默改写用户已落盘档位意图；依赖未启用的远程 catalog 才能选档
- required_evidence: `docs/research/thinking-levels-per-vendor-2026.md`（已附）
- refusal_contract: 不承诺兼容端 thinking ≡ 官方；未知 compat 不得瞎映射
- escalation_policy: 若需抬 session schema 版本或批量改写历史 level 字符串，须单独确认
