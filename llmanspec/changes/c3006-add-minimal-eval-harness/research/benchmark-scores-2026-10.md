# 官方评分与候选模型复核（2026-10-31）

> 性质：change 附属调研，不是 live spec。
> 范围：SWE-bench Verified / Terminal-Bench 4.0 / AA Intelligence Index 的公开分数，
> 以及 xylitol eval 的候选模型取舍。

## 一页结论

- 官方 SWE-bench Verified（bash-only / mini-SWE-agent）当前代表分数：Claude 4.5 Opus high
  76.8%、Gemini 3 Flash high 75.8%、MiniMax M2.5 high 75.8%、GLM 5 high 72.8%。
- 官方 Terminal-Bench 4.0：Opus 5.5(max) 64.8%、Sonnet 5.5(max) 61.8%、GPT-6 Astra(max)
  58.2%、GLM-5.3(max) 41.8%、GLM-5.3-Flash(none) 35.8%。
- 候选：GLM-5.3-Flash 做 dry-run；DeepSeek-V4.1-Flash-1017 做小集终验；同模型跑
  mini-SWE-agent baseline；前沿模型只作外部参照。
- 聚合站数值互相矛盾，不采用；只取官方榜与厂商文档。

## 官方 SWE-bench Verified

来源：<https://www.swebench.com/>（2026-10-31 读取；bash-only 行日期 2026-02-17）

| 模型 | Agent | % Resolved | Avg. $ | 日期 |
|---|---|---|---|---|
| Claude 4.5 Opus high | mini-SWE-agent | 76.8 | 0.75 | 2026-02-17 |
| Gemini 3 Flash high | mini-SWE-agent | 75.8 | 0.36 | 2026-02-17 |
| MiniMax M2.5 high | mini-SWE-agent | 75.8 | 0.07 | 2026-02-17 |
| Claude 4.6 Opus | mini-SWE-agent | 75.6 | 0.55 | 2026-02-17 |
| GLM 5 high | mini-SWE-agent | 72.8 | 0.53 | 2026-02-17 |
| GPT 5.2 high | mini-SWE-agent | 72.8 | 0.47 | 2026-02-17 |

Agent 条目（非 bash-only）：Claude 4.5 Opus + Sonar 79.2%；Doubao-Seed-Code + TRAE 78.8%；
Gemini 3 Pro + live-SWE-agent 77.4%。

口径提醒：官方榜是 bash-only / mini-SWE-agent，不是 xylitol 分数。xylitol 必须同模型、同数据、
同停止条件对比，报告写明 scaffold 差异。

## 官方 Terminal-Bench 4.0

来源：<https://www.tbench.ai/leaderboard>（2026-10-31 读取；行日期 2026-09）

| 模型 | Agent | 分数 | 日期 |
|---|---|---|---|
| Opus 5.5(max) | Claude Code | 64.8% ± 3.1 | 2026-09-22 |
| Sonnet 5.5(max) | Claude Code | 61.8% ± 2.9 | 2026-09-28 |
| GPT-6 Astra(max) | Codex | 58.2% ± 2.8 | 2026-09-03 |
| GLM-5.3(max) | Claude Code | 41.8% ± 3.2 | 2026-08-14 |
| GLM-5.3-Flash(none) | Claude Code | 35.8% ± 3.5 | 2026-08-26 |

## AA Intelligence Index v5.0

来源：<https://artificialanalysis.ai/models/glm-5-3-flash>、
<https://artificialanalysis.ai/models/deepseek-v4-1-flash-1017>（2026-10-31 读取）

| 模型 | AA Index | 速度 | 价格（in/out per 1M） | 上下文 |
|---|---|---|---|---|
| DeepSeek V4.1 Flash 1017 (Max) | 32 | 175 tok/s | $0.30 / $1.20 | 1M |
| GLM-5.3-Flash | 28 | 68.7 tok/s | $0.15 / $0.50 | 1M |

注意：GLM-5.3-Flash 厂商博客写 AA 57（v4.1.1）；当前 AA v5.0 是 28。版本不同，不能混用。

## 厂商自报

- DeepSeek-V4.1-Flash-1017：Terminal-Bench 4.0 42.3（Claude Code 框架）；DeepSWE v1.1 77.0；
  NL2Repo-Bench 68.2。来源：<https://api-docs.deepseek.com/news/news261017/>
- GLM-5.3-Flash：320B-A18B，1M 上下文，$0.15/$0.50；AA v4.1.1 57；$0.045/task（折扣）。
  来源：<https://z.ai/blog/glm-5.3-flash>

## 候选模型取舍

| 模型 | 角色 | 理由 | 风险 |
|---|---|---|---|
| GLM-5.3-Flash | dry-run / 工程验证 | 快、便宜、1M 上下文 | 需网关 key；TB4.0 35.8% |
| DeepSeek-V4.1-Flash-1017 | 小集终验 | AA 32、175 tok/s、1M 上下文；TB4.0 42.3 | 版本别名需确认 |
| GLM-5.3 | 强模型对照 | TB4.0 41.8% | 成本更高 |
| MiniMax M2.5 high | 廉价 SWE-bench 基线 | 75.8% @ $0.07 | 可用性未确认 |
| Gemini 3 Flash high | 对照 | 75.8% @ $0.36 | 可用性未确认 |
| Claude 4.5 Opus / Opus 5.5 | 外部前沿参照 | SWE-bench 76.8%；TB4.0 64.8% | 成本高，不做日常 |
| halogen-qwen3.8-flash-next | 本地免费冒烟 | 无网络、262K | 无基准分 |
| fake provider | 纯链路 | 确定性 | 不代表能力 |

## 聚合站不可用

benchlm、metatext、benchleader、benchgecko 对同一基准给出 96% / 79.2% / 76.8% / 93.9% 等
互相矛盾的数值。它们不作为依据；需要分数时只取官方榜与厂商文档。

## 对 eval 设计的影响

- 报告必须写 scaffold 差异（xylitol vs mini-SWE-agent），不能把官方榜分数当 xylitol 分数。
- 终验前先用 GLM-5.3-Flash 做 dry-run / 小集工程验证；确认后再用 DeepSeek-V4.1-Flash-1017。
- 模型 alias、key、网关地址只放本地 env / CI secrets；仓库只写占位符。
