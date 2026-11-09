---
brief: "Harbor eval baseline：固定模型/任务集/N=3、平台、并发、前后对比"
---

# Agent eval baseline（Harbor）

## 何时用

要判断 xylitol 改系统提示、工具策略或模型后是否真的进步时，先固定一份可复跑的
baseline；改完再用同一套命令重跑，比较 pass@1 / flip rate / cross-scaffold gap。
不要用单次主观体验代替 baseline。

## 基线定义（MUST）

- **固定模型**：`tools/eval/model-lock.yaml` 的 model id、API、context/output、
  thinking level；不要在一轮对比中换模型。
- **固定任务集**：当前典型集是 DeepSWE 5 task：`abs-module-cache-flags`、
  `actionlint-action-pinning-lint`、`adaptix-name-mapping-aliases`、
  `anko-typed-variable-bindings`、`arcane-drift-detection-baselines`。TB4 10 只作
  hard subset 记录，不跑 Pi；全量 20 task 以后再做。
- **固定 profile**：compare（`max_turns=80`、`wall_seconds=1800`）；smoke 只做冒烟。
- **固定 scaffold**：只保留 xylitol 与 Pi；mini-SWE 已移除。每 task × scaffold × N=3
  （典型值下限；DeepSWE 官方用 N=8，本地预算不足时 N=3，报告均值与离散度）。
- **固定 verifier**：只认 Harbor task verifier；不自研 judge，不混不同 scaffold/verifier。
- **固定并发**：本地单 job `-n 1`，全局最多 3 个并发 trial（`harbor_local.sh` 上限）；不要并行多个 job 叠并发。
- **固定记录**：baseline 结果写进 change research，不写进 live spec；skill 只讲方法。

## 平台

当前保留的 DeepSWE 原始镜像与 Pi 预装镜像都是单平台 `linux/amd64`：

- Linux x86_64：原生运行。
- macOS arm64：Docker Desktop 的 amd64 模拟（Rosetta/QEMU）。
- Linux arm64：需要 binfmt/qemu；不是默认支持平台，不要宣称开箱可跑。

不要加入「上游镜像没有 arm64、Harbor 又走 Dockerfile/Compose build」的基准：
SWE-bench Verified 就是因此移除的。Harbor Compose build 会按宿主平台拉取，
arm64 主机上直接报 `no matching manifest for linux/arm64/v8`。

验证单镜像平台：

```bash
docker run --rm --platform linux/amd64 <image> uname -m   # 期望 x86_64
```

## 跑法

日常入口：

```bash
just eval-typical xylitol
just eval-typical pi
just eval-typical-both
```

底层命令：

```bash
tools/eval/scripts/harbor_local.sh run -p <task-dir> -k 3 -n 1 \
  -a xylitol_agent:XylitolAgent -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_turns=80 --ak wall_seconds=1800 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -y
```

- `harbor_local.sh` 负责 proxy loopback 重写、`NO_PROXY`、拒绝含凭据 proxy URL、
  并发上限 3；并把 gateway host 加进 `--allow-agent-host`，否则 DeepSWE
  `no-network` task 的模型调用会被 egress sidecar 拒绝。
- Pi：DeepSWE `no-network` task 用预装镜像 + `preinstalled_agents:PiPreinstalled`：
  `-p /tmp/deep-swe-agents/<task> --effort high --ak model_api=openai-responses
  --ak max_turns=80 --ak disable_web_search=true
  --agent-timeout-multiplier 0.1666666667`；同时设 git identity。
- Pi 预装镜像 tag：`xylitol-eval/agents:<task>`；任务副本只改
  `[environment].docker_image`，verifier 镜像不变。
- DeepSWE no-network task 需要 egress sidecar（`gogost/gost`）；Docker Hub metadata
  不可达时先本地构建并打 tag。
- key 只走 `--env-file`；adapter 在容器内写 0600 `secret.env`，不把 key 放进
  `exec -e` argv。

## 典型值协议（跨 harness）

目标是让同一 model 在同一 task 上的分数反映 harness 差异，而不是采样噪声或环境差异。

- **N=3 起步**：每 task × scaffold 至少 3 次；报告 pass@1 均值、每 task 分布、flip rate。
  单次或 N=2 只能作 pilot，不作结论。
- **主集**：DeepSWE 5 task；这些 task 有 0/1 分布和近失，能区分 harness。
- **hard subset**：TB4 10 task 只保留 xylitol N=2 记录；不跑 Pi；floor effect
  明显，不并入主分。
- **同预算**：xylitol `wall_seconds=1800` + `max_turns=80`；Pi
  `max_turns=80` + `--agent-timeout-multiplier 0.1666666667`（Harbor 基准
  10800s × 0.1666666667 = 1800s；不要用 0.0625，只有 675s）。
- **环境对齐**：两个 scaffold 都设 git identity；xylitol 注入 CA；gateway host 进
  `--allow-agent-host`；Pi 关掉 web search（xylitol 无 web 工具）。
- **无效 trial 剔除**：quota 429、provider error、sidecar 平台错误、git identity
  导致空 patch 的 trial 标为无效并重跑，不混入均值。
- **报告**：per-task mean + overall mean + flip + cross-scaffold gap；给出 N 和
  无效 trial 数，不把不同 scaffold/verifier 的分数合并成一个数。

## 短程 smoke（框架维护）

每次改 eval 框架、Harbor adapter、binary 或配置后，先跑短程 smoke：

```bash
just eval-smoke --check   # 无模型调用；检查 Harbor、binary、env-file、fixture lint
just eval-smoke           # 1 trial、8 turns、120s；期望 reward=1
```

- smoke 只证明链路没断，不作为分数；失败先修框架，不跑全量。
- fixture 在 `tools/eval/fixtures/smoke-task/`，不依赖 `/tmp` 数据集。
- 模型额度耗尽时报告 `smoke blocked: model quota/rate limit`，不是框架失败。
- Pi 的兼容性另跑同 fixture 1 trial；不混分数。

## 长程节奏

- 一周一次 5-task 典型集；全量 20 task 以后再做。
- 典型集仍用同一 model lock、task 集、profile、verifier；`-n 1` 每个 scaffold。
- 每次典型集记录 commit、binary SHA、Harbor 版本、task checksums；manifest 不一致就标为新 baseline，不跨版本比较。

## 记录字段

- xylitol git commit、musl binary SHA-256、Harbor 版本。
- model id、profile、dataset revision、task ids。
- 每 trial reward；pass@1、flip rate、cross_scaffold_flip。
- cost、wall、turns；每个 scaffold / benchmark 单独一行。

## 下次对比

改完 xylitol 后用同一 task 集、同一 model、同一 profile、同一 verifier 重跑；
只比较同口径指标。baseline 记录在 change research，下一次结果追加到同一文档或
新日期文档，并注明 commit / binary sha。

## 检查清单

- [ ] model lock 未变；task ids 与上次一致。
- [ ] N=3；单 job `-n 1`；全局并发 ≤2。
- [ ] smoke 通过；失败先修框架再跑全量。
- [ ] Linux/macOS 平台假设已写明；镜像 `uname -m` 验证过。
- [ ] key 未进 git/argv；只走 env-file + 0600 secret.env。
- [ ] 结果写进 change research；未把临时 CI/PR 状态写进 issue。
