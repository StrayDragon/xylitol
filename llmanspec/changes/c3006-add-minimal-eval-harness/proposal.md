---
depends_on: []
title: Harbor-first Agent Eval（xylitol vs Pi）
needs_specs_change: false
---

# Harbor-first Agent Eval（xylitol vs Pi）

> **planned**：调研、设计与任务已收敛；按用户要求暂不建分支、不 push、不开 PR。
> 落实时走 apply；确认后再开 PR。

## Why

换模型、改系统提示或工具策略后，目前只能靠主观感受判断 xylitol 是否进步；没有可复现、
可审计的 cross-scaffold 对比。社区已有 Harbor verifier，不需要自研判分器。缺的是「固定模型 + 换 harness + 同数据同 verifier + N≥3」这条链路。

## What Changes

- `tools/eval/` 从“官方 SWE-bench 单 scaffold”改为 **Harbor-first**：
  - 固定模型 `group/glm-5.3-flash`；
  - `xylitol` 自定义 Harbor agent；`pi` / `mini-swe-agent` 用 Harbor 内置；
  - Terminal-Bench 4.0 10 + DeepSWE 10；
  - N=3；pass@1 + flip rate + cross_scaffold_flip；
  - Phase 1 只验证 xylitol 在 Harbor 跑通；Pi/mini-SWE 对照放 Phase 3。
- 新增 `tools/eval/harbor/xylitol_agent.py`：`BaseInstalledAgent`，上传 binary、
  写临时 config、headless `xylitol print`；reward 来自 Harbor task verifier。
- 保留既有 `tools/eval` 的 config / dataset / print runner / report 作为快速回归与
  legacy fallback；不纳入基准集。
- 新增仅 `workflow_dispatch` 的 Linux binary CI；只构建 static musl。
- 新增 `just` 配方（非默认门禁）；快速回归走 golden + replay，无模型调用。
- 不新增 live spec；工具契约写在 `tools/eval/README.md` 与本 change 的 `design.md`。
- 不改变产品默认行为；eval 是开发者/CI 后置能力，不进开箱体验。

## 决策

- **Harbor-first**：所有基准统一 Harbor task/verifier；DeepSWE 先 spike，不能稳定跑则
  只跑 Pier baseline，不宣称 Pi 对比。
- **固定模型**：同一轮对比中 xylitol 与 Pi 使用同一 model id、endpoint、effort、
  temperature、top_p、context、max turns/steps。
- **主表 B**：TB4.0 10 + DeepSWE 10；两者都在 Linux amd64 原生与
  macOS arm64 的 Docker amd64 模拟下运行。
- **N≥3**：pass@1、flip rate、cross_scaffold_flip、cost、wall、turns 分开报；
  不混不同 scaffold/verifier 的分数。
- **官方 verifier 为唯一分数源**；Langfuse 只作旁路元数据/失败实例回归，不作分数。
- **只经 `print` / headless 驱动**；禁止 PTY / 自动按键驱动 TUI 作为 eval 前置。
- **Python 编排**，不在 Rust runtime 引入 Docker/benchmark 依赖；Rust 侧只提供稳定 binary。
- **只用 magpie 提供的模型**；不使用 `DEEPSEEK_API_KEY`；模型 id 直接用 catalog 输出。
- **模型与密钥不外露**：仓库只写占位符；真实 key 只在本地 env / CI secrets；
  Harbor 用 `--env-file`，xylitol adapter 把 key 写入容器 0600 `secret.env`，
  `config.yaml` 只引用 `{{ secret.MAGPIE_GATEWAY_KEY }}`；不放进 `exec -e` argv。
- **确认前不开 PR、不 push**；避免未审查代码与密钥泄露。

## Capabilities

- 无 live spec 行为变更（`needs_specs_change: false`）；eval 是开发工具链。
- 若后续新增 runtime 停止条件（cost/token），届时按 `agent-runtime` / `runtime-config`
  另开 SDD change。

## Impact

- 新增 `tools/eval/harbor/xylitol_agent.py`；调整 `tools/eval` 文档/示例/任务。
- 新增 Harbor 依赖说明（临时 venv / CI 安装）；不改产品线协议/TUI/Print 行为。
- 新增快速回归 fixtures（Phase 4）。

## 阶段

| 阶段 | 交付 | 验收 |
|---|---|---|
| Phase 0 | `tools/eval` 骨架 + 本地 binary | 单测/dry-run/本地 binary |
| Phase 1 | Harbor adapter + xylitol 单 task | agent 跑通；verifier 产出 reward |
| Phase 2 | TB4.0 单 task + DeepSWE spike + musl | 两个基准各 1 task 跑通 |
| Phase 3 | Pi + mini-SWE + 10/10 N=3 | pass@1 + flip rate 表 |
| Phase 4 | golden + replay + snapshot | PR 门禁可跑 |
| Phase 5 | CI / Langfuse 旁路 / 云 | 回归与全量可复现 |

## 风险

- magpie catalog 只有 7 个模型；最新为 GLM-5.3 家族，无 Opus/GPT/K3；需要时先扩网关。
- `group/glm-5.3-flash` pricing 未返回；成本以 usage 为准。
- Docker Hub 不稳定；Harbor compose 可能卡 registry metadata；本地用
  `DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=0`。
- TB4.0 含 GPU/多容器任务；本地先排除。
- DeepSWE Harbor 兼容性未验证；先 spike，失败则只跑 Pier baseline。
- SWE-bench Verified 已移除：上游镜像只有 linux/amd64，Harbor Compose build 在
  arm64 主机上按宿主平台拉取会失败；保留的 TB4.0 / DeepSWE 镜像同为 amd64，
  在 macOS arm64 走 Docker amd64 模拟。
- 官方榜口径与 xylitol 不同；报告必须写明 scaffold 与 verifier。

## Further Notes

- 2026-11-01 Harbor-first 调研：[research/harbor-first-eval-2026-11.md](./research/harbor-first-eval-2026-11.md)。
- 2026-10-31 官方评分与候选模型复核：[research/benchmark-scores-2026-10.md](./research/benchmark-scores-2026-10.md)。
- GitHub issue：[#21](https://github.com/StrayDragon/xylitol/issues/21)。
