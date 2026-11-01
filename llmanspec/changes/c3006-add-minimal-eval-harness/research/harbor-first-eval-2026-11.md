# Harbor-first eval 调研（2026-11-01）

> 性质：change 附属调研，不是 live spec。
> 范围：Pi / DSH / OpenCode 的 eval 手段，Harbor、SWE-bench Verified、Terminal-Bench 4.0、
> DeepSWE 的现状，以及 xylitol 的调整决策。

## 结论

- SWE-bench Verified 已高度饱和（官方头部约 93.9%），不适合作为唯一能力闸；
  保留 25 task 作回归/兼容。
- 能力闸改为 Terminal-Bench 4.0（66 task）+ DeepSWE v1.1（113 task）；都用 Harbor
  task/verifier；DeepSWE 先做 Harbor spike。
- 同一模型 `group/glm-5.3-flash` 分别跑 xylitol 与 Pi；N=3；报 pass@1 与 flip rate。
- Phase 1 只验证 xylitol 在 Harbor 跑通；Pi/mini-SWE 对照放 Phase 3。
- magpie catalog 只有 7 个模型；最新为 GLM-5.3 家族，无 Opus/GPT/K3；需要时先扩网关。

## Pi

来源：[Pi evals README](https://github.com/earendil-works/pi/blob/13cbf77df2396303013a41646bcfa77b4271ae56/packages/evals/README.md)

- `vitest-evals`；`*.docs.eval.ts` 做 without_docs / with_docs Docker A/B，报 lift；
  其他 `*.eval.ts` 是本机 host evals（smoke、docs audit）。
- 每个 `(case, variant, repetition)` 在 fresh container 跑；支持 `--runs-per-variant`；
  缺 arm 会 block 配对，不把缺失当 0。
- 保留 session JSONL；公开 SWE/TB 多在外部对照，CHANGELOG 很少写“SWE +x”。

可抄：小 golden + 多 run + session 快照；docs A/B 只用于文档改动，不作主能力闸。

## DSH

来源：[DeepSeek-V4.1-Flash 模型卡](https://huggingface.co/deepseek-ai/DeepSeek-V4.1-Flash)

- 固定模型，横向 Claude Code / Codex / OpenCode / Pi / mini-SWE / DSH Minimal/Standard/PTC。
- DeepSWE v1.1 resolved：Claude Code 69.8、Codex 65.6、OpenCode 65.5、Pi 66.2、
  mini-SWE 74.2、DSH Minimal 72.6、DSH Standard 70.5、DSH PTC 67.6。
- Terminal-Bench 2.1 Pass@1：88.0 / 84.1 / 85.0 / 86.1 / 90.3 / 90.6 / 85.8 / 85.8。
- N=8（DeepSWE）/ N=3（TB2.1）；temp 1.0、top_p 0.95、1M context、max_steps=500；
  TB2.1 无网络。
- 另有 unit、real-API smoke、session snapshot / traj replay。

可抄：固定模型 + 多 scaffold 表 + N 次重复；不抄单次全量结论。

## OpenCode

来源：[OpenCode Bench](https://github.com/anomalyco/opencode-bench)

- 真实生产 commit；3 个隔离 episode；30 分钟超时；3 个 LLM judge、五维加权、
  disagreement penalty。
- 没有内置 SWE/TB nightly；judge 噪声明显。
- 可抄真实 fixture + episode 隔离；硬判改为测试命令，不用多 LLM judge。

## Harbor

来源：[Harbor](https://github.com/laude-institute/harbor)、
[自定义 agent](https://docs.harborframework.com/agents/custom-agents)、
[预集成 agent](https://docs.harborframework.com/agents/pre-integrated-agents)、
[SWE-bench adapter](https://github.com/laude-institute/harbor/tree/main/adapters/swebench)。

- Terminal-Bench 官方 harness；支持 `terminal-bench/terminal-bench@4.0.0`（66 task）。
- SWE-bench Verified adapter：500 task；parity 记录中 mini-swe-agent 2.1.0 跑 3 次，
  499 comparable task，Harbor 0.545 vs 官方 0.563。
- 内置 `pi`、`mini-swe-agent`、`claude-code`、`codex`、`opencode` 等。
- 自定义 agent 用 `BaseInstalledAgent`；`-a module.path:Class`。
- 支持 `-k` attempts、`-n` 并发、`-l` 限 task、`--effort`、`--ae`/`--ak`、
  `--env-file`、`--base-url`、`--api-key-name`、`--api-format`。

## Terminal-Bench 4.0

来源：[EvalScope TB4.0](https://evalscope.readthedocs.io/zh-cn/latest/benchmarks/terminal_bench_v4.html)、
[Harbor release](https://github.com/harbor-framework/terminal-bench/releases)。

- 66 task；固定 4.0.0；11 个多容器；3 个 H100 GPU task；每 task 8h agent 超时。
- 独立 verifier 环境；reward = 有效官方 verifier 0/1 均值。
- 本地先排除 GPU/多容器；只跑可承担的 10 task 子集。

## DeepSWE v1.1

来源：[DeepSWE](https://github.com/datacurve-ai/deep-swe)、[Pier](https://github.com/datacurve-ai/pier)。

- 113 task、91 repo、5 languages；Harbor task format。
- v1.1 用 Harbor separate verifier + `[[verifier.collect]]`；官方榜用 Pier + mini-swe-agent on Modal。
- Pier agents 不含 Pi；因此 DeepSWE 的 Pi 对比需先验证 Harbor 能否直接跑 task；
  不能则只跑 Pier baseline，不混表。
- 子集：`pier run -p deep-swe/tasks --agent mini-swe-agent --n-tasks 10 --sample-seed 0`。

## SWE-bench Verified / Pro

来源：[SWE-bench](https://www.swebench.com/)、
[Startrise 2026-10 综述](https://www.startrise.io/blog/llm-coding-benchmarks/)、
[SWE-bench Pro](https://labs.scale.com/leaderboard/swe_bench_pro_public_v2)。

- Verified 500 task；头部约 93.9%，已饱和；保留 25 task 作回归/兼容。
- Pro 公开集 731 task；Epoch 审计认为约 30% task 有缺陷；暂不作主闸。

## Magpie catalog（2026-11-01）

| id | context | max output | reasoning | pricing |
|---|---|---|---|---|
| `group/glm-5.3-flash` | 1M | 128000 | low/high/max | 未返回 |
| `zcode/GLM-5.3` | 1M | 128000 | low/high/max | $1.19516 / $4.18306 |
| `zcode/GLM-5.3-Flash` | 1M | 128000 | low/high/max | $0.11952 / $0.41831 |
| `fusionxparks/DeepSeek-V4.1-Flash-1017` | 512K | 32000 | 无 | $0.15 / $0.60 |
| `cursor/deepseek-v41-flash-1017` | 1M | 未返回 | none/low/medium/high/max | 未返回 |
| `tufa-halogen/halogen-qwen3.8-flash-next` | 256K | 32000 | low/medium/xhigh | 本地免费 |

`group/glm-5.3-flash` 实测 `openai-responses` 可用；max output 128000；
thinking 使用 `[low, high]`，默认 high（列表最后一项）。

## 决策

1. 主表 B：SWE Verified 25（回归）+ TB4.0 10 + DeepSWE 10。
2. 固定模型 `group/glm-5.3-flash`；xylitol 与 Pi 同模型同配置。
3. Harbor-first；Phase 1 只做 xylitol 单 task。
4. N=3；pass@1 + flip rate + cross_scaffold_flip；不混 scaffold/verifier。
5. 快速回归：golden + replay + session snapshot；无模型。

## Phase 1 实测

- 2026-11-01：Harbor 0.25 + `xylitol_agent:XylitolAgent` + `group/glm-5.3-flash`，
  跑 `swe-bench/swe-bench-verified` 的 `astropy__astropy-12907`。
- 结果：reward 1.0；verifier `PASSED`；12m55s；0 exceptions。
- 本地 job：`jobs/xylitol-harbor-swe-astropy-4`（ignored）。
- Docker Hub 不稳；本地用 `DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=0`。
- key 写入容器 0600 config，不在 `exec -e` argv。

## Phase 2 实测

- static musl：`x86_64-unknown-linux-musl`，`ldd` 显示 `statically linked`，无 interpreter；
  sha256 `9f5c4f04f64bc914fe0cfdd5d5355c4bf28bdada1c69077599f33d1bbaa22fde`。
- Terminal-Bench 4.0 `layout-config-recreation`（单容器/CPU）：reward 0.0；
  verifier 11 failed / 1 passed；10m33s；0 exceptions。
- DeepSWE v1.1 `httpx-multipart-response-parsing`：reward 0.0；F2P 0.0；P2P 1.0；
  partial 0.912；10m41s；0 exceptions。Harbor 0.25 能跑 schema 1.3 +
  `environment_mode=separate` + `[[verifier.collect]]`。
- DeepSWE `agent.network_mode=no-network` 触发 Harbor egress sidecar；Docker Hub
  metadata 拉取失败时，先用本地 `gogost/gost` 构建并打 tag
  `harbor-prebuilt:harbor-docker-egress-control-sidecar--e9f39ce1643ee690`，再重跑。
- model/config lock：`tools/eval/model-lock.yaml`；`doctor --lock` 校验。
- 结论：三个基准各 1 task 均能跑通 agent + verifier；smoke 不要求通过。

## 风险

- `group/glm-5.3-flash` pricing 未返回；成本以 usage 为准。
- Docker Hub 不稳定；Harbor compose 可能卡 registry metadata；本地用
  `DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=0`。
- TB4.0 GPU/多容器 task 本地不可跑。
- DeepSWE Harbor 兼容性未验证；先 spike。
- glibc binary 在非 Ubuntu task 可能不兼容；Phase 2 加 static musl。
- magpie 无 Opus/GPT/K3；如要最新前沿模型需扩网关。
