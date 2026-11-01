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
- **固定任务集**：当前 baseline 是 TB4.0 10 + DeepSWE 10；具体 task ids 记录在
  `llmanspec/changes/<id>/research/baseline-YYYY-MM.md`，skill 不复制易腐清单。
- **固定 profile**：compare（`max_turns=80`、`wall_seconds=1800`）；smoke 只做冒烟。
- **固定 scaffold**：xylitol、Pi、mini-SWE 同 task 同 verifier；每 task × scaffold × N=3。
- **固定 verifier**：只认 Harbor task verifier；不自研 judge，不混不同 scaffold/verifier。
- **固定并发**：本地单 job `-n 3`，全局最多 3 个并发 trial；不要并行多个 job 叠并发。
- **固定记录**：baseline 结果写进 change research，不写进 live spec；skill 只讲方法。

## 平台

保留的 TB4.0 / DeepSWE 镜像都是单平台 `linux/amd64`：

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

本地入口：

```bash
tools/eval/scripts/harbor_local.sh run -p <task-dir> -k 3 -n 3 \
  -a xylitol_agent:XylitolAgent -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_turns=80 --ak wall_seconds=1800 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -y
```

- `harbor_local.sh` 负责 proxy loopback 重写、`NO_PROXY`、拒绝含凭据 proxy URL、
  并发上限 3。
- Pi：`-a pi --effort high --ak model_api=openai-responses`；nvm 安装走 proxy。
- mini-SWE：`-a mini_swe_agent_mcp:MiniSweAgentWithMcp --ak version=2.1.0
  --agent-setup-timeout-multiplier 3`；Harbor 安装器缺 `mcp`，用自定义 agent。
- DeepSWE no-network task 需要 egress sidecar（`gogost/gost`）；Docker Hub metadata
  不可达时先本地构建并打 tag。
- key 只走 `--env-file`；adapter 在容器内写 0600 `secret.env`，不把 key 放进
  `exec -e` argv。

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
- [ ] N=3；单 job `-n 3`；全局并发 ≤3。
- [ ] Linux/macOS 平台假设已写明；镜像 `uname -m` 验证过。
- [ ] key 未进 git/argv；只走 env-file + 0600 secret.env。
- [ ] 结果写进 change research；未把临时 CI/PR 状态写进 issue。
