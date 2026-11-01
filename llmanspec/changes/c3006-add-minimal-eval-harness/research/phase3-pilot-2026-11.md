# Phase 3 pilot（2026-11-01）

> 性质：change 附属调研；只记录 smoke 级对比，不是 25/10/10 全量结论。

## 目的

验证同一模型 `group/glm-5.3-flash` 在三种 scaffold（xylitol / Pi / mini-SWE）
上使用同一 SWE-bench Verified task、同一 Harbor verifier、N=3 的指标链路。

## 配置

- Task：`swe-bench/swe-bench-verified` 的 `astropy__astropy-12907`。
- Model：`openai/group/glm-5.3-flash`；`--api-format openai_responses`；`--effort high`。
- N=3；`-n 3`；max_turns 8；wall 600s；smoke profile。
- xylitol：glibc binary；`xylitol_agent:XylitolAgent`。
- Pi：`-a pi`；`--ak model_api=openai-responses`；`--ak max_turns=8`；
  `--ae HTTPS_PROXY=http://host.docker.internal:20171`（nvm 安装需要）。
- mini-SWE：`mini_swe_agent_mcp:MiniSweAgentWithMcp`；`--ak version=2.1.0`；
  `--agent-setup-timeout-multiplier 3`；`--ae HTTPS_PROXY=...`。

## 结果

| scaffold | task | runs | rewards | pass@1 | flip |
|---|---|---|---:|---|---:|
| xylitol | astropy__astropy-12907 | 3 | 1, 1, 1 | 1.000 | no |
| pi | astropy__astropy-12907 | 3 | 1, 1, 1 | 1.000 | no |
| mini-swe | astropy__astropy-12907 | 3 | 1, 1, 1 | 1.000 | no |

- Cross-scaffold：mean gap 0.000；disagree tasks 0；disagree rate 0.000。
- 三个 scaffold 均 0 exceptions。
- wall（agent execution 平均）：xylitol 134.2s；Pi 76.9s；mini-SWE 121.9s。
- cost：Harbor 未返回 usage；`n/a`。

## 兼容性发现

- Pi 安装 Node 走 nvm；容器内需把宿主代理改写为
  `http://host.docker.internal:20171`，否则 nvm clone 可能 TLS 失败。
- mini-SWE 的 LiteLLM 会 import `mcp`；Harbor 安装命令未带该包。
  自定义 `MiniSweAgentWithMcp` 在 super install 后把 `mcp` 装进 tool venv。
- 三者都能走 `openai_responses`；mini-SWE 不需要退回 chat completions。
- Pi 的 `max_turns=8` 会在修复后 abort；本次仍拿到 reward 1.0，但全量对比
  应用 `compare` profile 的 80 turns，避免过早截断。

## 限制

- 只有 1 个 SWE Verified task；不是 25 task 子集。
- TB4.0 / DeepSWE 未做 cross-scaffold；Phase 2 只有 xylitol 单 task。
- 单 task 全 1.0，无法产生 flip；不能据此宣称 scaffold 差异。
- 全量 25/10/10 × N=3 预计需要数小时并发运行；需单独预算窗口。

## 下一步

1. 用 compare profile（max_turns 80 / wall 1800）重跑 pilot，确认无截断。
2. 跑 SWE Verified 25 / TB4.0 10 / DeepSWE 10 × N=3。
3. 用 `tools/eval/scripts/summarize_harbor.py` 汇总 pass@1 / flip /
   cross_scaffold_disagree。
