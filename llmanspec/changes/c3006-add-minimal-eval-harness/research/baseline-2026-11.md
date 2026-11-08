# Baseline：TB4.0 10 + DeepSWE 10（2026-11）

> 性质：change 附属基线记录，不是 live spec。
> 目的：固定这一轮评测集与命令；xylitol 功能改进后按同一命令重跑，比较提升。
> 复用方法：`xylitol-dev` skill 的 `references/eval-baseline.md`。

## 范围

- 基准：Terminal-Bench 4.0 10 task + DeepSWE v1.1 10 task。
- Scaffold（历史）：xylitol、Pi、mini-SWE；当前只保留 xylitol + Pi。
- 每个 task × 每个 scaffold × N=3。
- 模型：`group/glm-5.3-flash`；`openai-responses`；thinking high。
- Profile：compare，`max_turns=80`、`wall_seconds=1800`。
- Harbor 0.25；本地顺序执行，单 job `-n 3`，全局最多 3 个并发 trial。

## 平台

- Linux x86_64：原生运行。
- macOS arm64：Docker Desktop 的 amd64 模拟（Rosetta/QEMU）。
- 保留的 TB4.0 / DeepSWE 镜像都是单平台 `linux/amd64`。
- Linux arm64 需要 binfmt/qemu；不是默认支持平台。
- 已验证：DeepSWE 镜像在 macOS arm64 上以 `--platform linux/amd64` 运行，
  `uname -m` 返回 `x86_64`；TB4.0 `layout-config-recreation` 的 3 个 trial
  已在 macOS arm64 上跑完（reward 0.0）。
- SWE-bench Verified 已移除：上游镜像只有 `linux/amd64`，Harbor Compose
  build 在 arm64 主机上按宿主平台拉取，manifest 没有 arm64 条目。

## Task 集

Terminal-Bench 4.0（单容器/CPU）：

```text
layout-config-recreation
atrx-vep-crispr
batched-eval-parity
biped-contact-dynamics
bun-sourcemap-leak
cad-model
cargo-flight-dispatch
coq-block-bound
ctr-optimization
data-anonymization
```

DeepSWE v1.1：

```text
abs-module-cache-flags
abs-stepped-slices
actionlint-action-pinning-lint
adaptix-name-mapping-aliases
aiomonitor-task-snapshots-diff
anko-default-function-arguments
anko-typed-variable-bindings
arcane-drift-detection-baselines
arktype-json-schema-refs-dependencies
awilix-async-container-initialization
```

## 命令

单 task 示例：

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation -k 3 \
  -a xylitol_agent:XylitolAgent -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_turns=80 --ak wall_seconds=1800 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -n 3 -y
```

Pi：

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation -k 3 \
  -a pi -m openai/group/glm-5.3-flash --effort high \
  --ak model_api=openai-responses --ak max_turns=80 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -n 3 -y
```

mini-SWE：

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation -k 3 \
  -a mini_swe_agent_mcp:MiniSweAgentWithMcp -m openai/group/glm-5.3-flash \
  --effort high --ak version=2.1.0 --agent-setup-timeout-multiplier 3 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -n 3 -y
```

DeepSWE 把 `-p` 换成对应 task 目录；no-network task 需要 Harbor egress
sidecar（`gogost/gost`）。

## 记录字段

- xylitol git commit；musl binary SHA-256
  `9f5c4f04f64bc914fe0cfdd5d5355c4bf28bdada1c69077599f33d1bbaa22fde`。
- Harbor 版本、dataset revision、model id、profile、task ids。
- 每 trial reward、pass@1、flip rate、cross_scaffold_flip、cost、wall、turns。
- 每个 scaffold / benchmark 单独一行；不合并成单一分数。

## 状态

2026-11-01：baseline 已启动（TB4.0 10 + DeepSWE 10 × N=3 × 3 scaffolds），
结果待收。首轮 SWE 25 已因 arm64 平台问题作废，不计入 baseline。

## 2026-11-07 复跑前置（preflight）

- 代码：main `0aeed34`；距 2026-11-02 的 smoke 合并约 262 个提交。主要变化：MCP
  OAuth / elicitation / prompts / resources / capability probe；codemode 构建与
  工具选择；`models.entries` 与 `thinking_level_map` 配置；session profile；
  trust 加固。headless `xylitol print` 仍走 adapter 的 config，未装配 MCP。
- 工具：Harbor 0.25.0 重建于 `/tmp/harbor-venv`；TB4 66 task 与 DeepSWE 117
  task 源已下载到 `/tmp/tb4`、`/tmp/deep-swe`。
- binary：`64f7a81049b39244fbc1137b3b60b8d6059ac28f9ef85035054a58c8853a01fa`，
  39MB，`static-pie linked`；smoke reward=1。
- 镜像：TB4 10 task 的 env+verifier 共 20 个，约 10.2GB；DeepSWE 10 个，约
  9.6GB；已全部拉到本地，平台均 `linux/amd64`；宿主剩余约 44GB。
- Task 集调整：`batched-eval-parity` 在 macOS Docker 下报
  `network_mode='no-network' is not supported by EnvironmentType.DOCKER`，
  替换为单容器 `session-window-debug`。`bun-sourcemap-leak` 的环境镜像没有
  CA bundle；adapter 注入 certifi bundle 并设 `SSL_CERT_FILE`，短跑已确认模型
  调用正常（reward 0 是 4 turns 内未完成 task，不是 TLS 失败）。
- 建议跑法：先 xylitol 20 task × N=2（compare profile，全局并发 2）；结果出来
  后再决定 Pi/mini 子集。仍不混 scaffold/verifier。

## 2026-11-08 xylitol TB4 baseline（N=2）

- 10 task × N=2 = 20 trials；0 exceptions。
- pass@1 = 0.050；flip rate = 0.100；平均 wall = 1456.1s；11/20 trial
  wall ≥1790s（多数撞 1800s 上限）。
- 仅 `atrx-vep-crispr` 有 1/2 通过（reward 0, 1）；其余 9 task 均 0/2。
- `layout-config-recreation` 0/2（此前 0/3）；`atrx` 1/2（此前 2/3）。两次
  N 不同，只能记方向，不作强弱结论。
- `bun-sourcemap-leak` 已能正常调用模型（CA 注入生效），但 2 次均未通过 task。
- DeepSWE 首次全量结果作废：`no-network` task 未放行 gateway host `tufa`，
  agent log 报 `provider error: ... http://tufa:50888/v1/responses`；已修
  `harbor_local.sh` 的 `--allow-agent-host`，10 task × N=2 重跑中。

## 2026-11-08 DeepSWE baseline（N=2）

- 10 task × N=2 = 20 trials；0 exceptions。
- pass@1 = 0.450；flip rate = 0.500；平均 wall = 1366.6s；6/20 trial
  wall ≥1790s。
- 2/2：`abs-stepped-slices`、`aiomonitor-task-snapshots-diff`。
- 1/2：`abs-module-cache-flags`、`actionlint-action-pinning-lint`、
  `adaptix-name-mapping-aliases`、`anko-typed-variable-bindings`、
  `arcane-drift-detection-baselines`。
- 0/2：`anko-default-function-arguments`、`arktype-json-schema-refs-dependencies`、
  `awilix-async-container-initialization`。
- `adaptix` 的一个 0 trial 在最终 `git commit` 时因容器没有 git identity 失败，
  verifier 看到空 patch；记为 task 失败，不是 harness 异常。
- 合并 TB4 + DeepSWE：40 trials，10 pass，整体 pass@1 0.25；TB4 0.05，
  DeepSWE 0.45。

## 2026-11-08 典型值协议调整

- 2026-11-08 的 N=2 结果只作 pilot；典型值改为 N=3 起步。
- 主集改为 DeepSWE 10（有 0/1 分布与近失，能区分 harness）；TB4 10 作 hard
  subset 单独报，floor effect 明显，不并入主分。
- 同预算：xylitol 1800s / 80 turns；Pi `max_turns=80` + agent timeout
  1800s；mini-SWE `tools/eval/mini-swe-eval.yaml`（`step_limit=80`、
  `wall_time_limit_seconds=1800`）+ 同 timeout。
- 环境对齐：所有 scaffold 设 git identity；Pi/mini 关 web search；xylitol
  注入 CA；gateway host 进 allowlist。
- 报告 per-task mean、overall mean、flip、cross-scaffold gap；剔除无效 trial
  并重跑。

## 2026-11-08 预算收口（N=3）

- 不做 N=5；N=3 作为本轮典型值验证。
- xylitol N=3 跑 DeepSWE 10（已启动），其中 5 个 task 用于 cross-scaffold。
- Pi / mini 只跑 5 个 DeepSWE task：`abs-module-cache-flags`、
  `actionlint-action-pinning-lint`、`adaptix-name-mapping-aliases`、
  `anko-typed-variable-bindings`、`arcane-drift-detection-baselines`。
- 每个 scaffold 5 × 3 = 15 trials；两 scaffold 合计 30 trials。
- TB4 不跑 Pi/mini；TB4 只保留 xylitol N=2 hard subset。

## 2026-11-08 xylitol DeepSWE 5-task N=3

- 5 task × N=3 = 15 trials；0 exceptions；7/15 pass；pass@1 = 0.467；
  flip rate = 0.600。
- `abs-module-cache-flags` 3/3；`actionlint-action-pinning-lint` 1/3；
  `adaptix-name-mapping-aliases` 0/3；`anko-typed-variable-bindings` 1/3；
  `arcane-drift-detection-baselines` 2/3。
- 与 N=2 对比：`abs-module` 1/2 → 3/3；`adaptix` 1/2 → 0/3；N=2 噪声明显。
- Pi / mini 的 5 task × N=3 队列已启动；每 scaffold 15 trials。

## 2026-11-08 DeepSWE no-network 安装问题

- Pi / mini 第一次 N=3 全部失败：DeepSWE `no-network` 阻断 nvm / uv 安装，
  `raw.githubusercontent.com` / GitHub / PyPI 不可达；结果作废。
- 解决：从 5 个 DeepSWE task 镜像构建 `xylitol-eval/agents:<task>`，预装 Pi 与
  mini-SWE；新增 `tools/eval/harbor/preinstalled_agents.py`，install 改为 no-op。
- 任务副本在 `/tmp/deep-swe-agents/<task>`，只改 `[environment].docker_image`，
  verifier 镜像不变。
- Pi pilot：0 exceptions，1m50s；mini pilot：agent 在 288s 短上限下超时，链路已启动。
- 正式 Pi / mini N=3 已用预装镜像启动。

## 2026-11-08 收口（当前口径）

- 只保留 xylitol + Pi；mini-SWE 已移除。
- 只跑 5-task DeepSWE typical set，N=3；TB4 只保留 xylitol N=2 hard subset 记录。
- 镜像只保留 5 个 DeepSWE 原始镜像 + 5 个 Pi 预装镜像 + egress sidecar；
  TB4 20 个镜像与 DeepSWE 未用 5 个已删除。
- 日常入口：`just eval-typical xylitol` / `just eval-typical pi` /
  `just eval-typical-both`；框架维护仍用 `just eval-smoke --check`。

## 2026-11-08 Pi 超时口径修正

- `--agent-timeout-multiplier 0.0625` 对应 Harbor 基准 10800s × 0.0625 = 675s，
  不是 1800s；首轮 Pi N=3 因 675s 超时作废。
- 改为 `0.1666666667`，即 1800s，与 xylitol `wall_seconds=1800` 对齐。
- Pi N=3 已用新口径重启。

## 下一次对比

改进 xylitol 后，用同一 task 集、同一模型、同一 profile、同一 verifier 重跑；
比较 pass@1、flip rate、cross_scaffold_flip。不得混不同 scaffold/verifier 的分数。
