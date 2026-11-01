# Baseline：TB4.0 10 + DeepSWE 10（2026-11）

> 性质：change 附属基线记录，不是 live spec。
> 目的：固定这一轮评测集与命令；xylitol 功能改进后按同一命令重跑，比较提升。
> 复用方法：`xylitol-dev` skill 的 `references/eval-baseline.md`。

## 范围

- 基准：Terminal-Bench 4.0 10 task + DeepSWE v1.1 10 task。
- Scaffold：xylitol、Pi、mini-SWE（`mini_swe_agent_mcp:MiniSweAgentWithMcp`）。
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

## 下一次对比

改进 xylitol 后，用同一 task 集、同一模型、同一 profile、同一 verifier 重跑；
比较 pass@1、flip rate、cross_scaffold_flip。不得混不同 scaffold/verifier 的分数。
