# Agent Eval 最小底座调研（#21）

> 调研日期：2026-10-30
> 性质：research / 选型底稿，不是 live spec；结论可回写 proposal / design。
> 范围：xylitol 的 headless coding-agent eval 底座，不含 TUI 交互评测。
> 2026-10-31 复核：官方评分与候选模型见 [benchmark-scores-2026-10.md](./benchmark-scores-2026-10.md)；harness 版本与数据集组合以 `design.md` 为准。

## 0. 一页结论

- **最小底座 = 官方 grader + headless `xylitol print` + 每实例 Docker + predictions JSONL。**
  不要自研判分器，也不要用 Langfuse judge 当分数。
- **M0 先跑 SWE-bench Verified 5 实例**：用官方 SWE-bench 镜像/预构建镜像，agent 在容器内改仓库，
  容器外收集 `git diff` 成 `preds.jsonl`，再交给官方 `swebench.harness.run_evaluation` 出
  `results.json`。验收是「5/5 被 grade、resolved 数可复现」，不是「跑出高分」。
- **M1 补 eval profile**：`session.max_turns` 已有；还缺墙钟超时、成本/token 停止、eval 配置锁定、
  禁止问用户、超限 autosubmit 交卷协议。
- **M2 扩到 Verified ~50**，同口径跑 mini-SWE-agent 基线；报告写明 xylitol 版本、binary hash、
  eval profile、model id、harness 版本、是否 pass@1 / best-of-N。
- **M3 再接 Terminal-Bench**：当前官方是 **Terminal-Bench 4.0（66 tasks）**，由 **Harbor** 承载；
  issue 里的「2.x」已过时。Harbor 用 `BaseInstalledAgent` 适配 xylitol，reward 由 task 的
  `tests/test.sh` 写 `/logs/verifier/reward.txt|json`。
- **Artificial Analysis 只读**：Coding Agent Index v2.0 是 FrontierCode v1.1 + Terminal-Bench 4.0 +
  SWE-Atlas-QnA-AA 的等权平均，且用 AA 自己的 harness；不能把 AA 分数当 xylitol 分数。

**ROI 排序**：M0 > M1 > M2 > M3 > M4 > M5。M0/M1 是「能出可信分」的必要条件；M3 依赖 Harbor
适配器，成本明显更高；M5 全量云跑在 M2 稳定前不做。

## 1. 问题与边界

#21 要回答的是「换模型、改系统提示或工具策略后，xylitol 是否进步」，不是做通用评测平台。
因此底座必须满足：

- 分数来自社区官方 grader（SWE-bench `run_evaluation` / Harbor verifier），不是自造 judge。
- 驱动面只经 `print` / headless（`XyDriver::run`），禁止 PTY / 自动按键驱动 TUI。
- 每实例隔离、可复现、可审计：记录 dataset revision、instance ids、binary hash、config hash、
  harness 版本、model id、停止条件。
- 失败实例能沉淀成小回归集，防同一 bug 再犯。
- 默认不开箱；开发者/CI 后置。

非目标：

- 不做多用户评测平台 / leaderboard 运营。
- 不用 TUI 交互或自动按键跑 benchmark。
- 不用 Langfuse Experiments 当主分数。
- 不把竞赛题（LiveCodeBench 等）替代 repo / 终端 agent eval。
- 不在 M0 就做全量并行 / 云调度。

## 2. 外部事实

### 2.1 SWE-bench

- 官方评估流程：模型对每个 instance 生成 patch；harness 在 Docker 中应用 patch、跑仓库测试，
  判定 resolved。入口是 `python -m swebench.harness.run_evaluation`。
  <https://github.com/swe-bench/swe-bench/blob/main/docs/guides/evaluation.md>
- 数据集规模（官方文档）：Lite test 300；Verified test 500；full dev 225；full test 2294。
  <https://github.com/swe-bench/swe-bench/blob/02e7a74ffd0b707aab73d203fe87bdc7c76afc8e/docs/assets/evaluation.md>
- prediction 格式是 JSONL，每行：
  `{"instance_id", "model_name_or_path", "model_patch"}`；`model_patch` 是 unified diff 字符串。
- 官方 harness 产物：`logs/evaluation/<run_id>/results.json` + 每实例
  `report.json` / `test_output.txt` / `run_instance.log` / `eval.sh` / `patch.diff`。
  `results.json` 区分 submitted / completed / resolved / unresolved / empty patches / errors /
  likely infrastructure failures。`0 failed` 不等于没有 error，要看 `Instances with errors`。
- 官方 CLI 现在也提供 `swebench infer` / `swebench eval` / `swebench report`；`infer` 是
  mini-SWE-agent 的薄封装，`eval` 用官方 harness 判分。
  <https://www.swebench.com/SWE-bench/reference/cli/>
- Docker 镜像：官方/社区有预构建 instance images；Epoch AI 镜像命名
  `ghcr.io/epoch-research/swe-bench.eval.<arch>.<instance_id>`，Verified x86_64 500/500 可用。
  <https://github.com/epoch-research/SWE-bench>
- 官方 harness 支持 `--instance_ids`、`--max_workers`、`--cache_level`、`--clean`；
  `run_id` 相同会复用缓存，改 patch 必须换 `run_id`。

### 2.2 mini-SWE-agent

- 定位：100 行左右的 bash-only agent，线性历史、每个 action 独立 `subprocess.run`；
  官方宣称 SWE-bench Verified >74%（README 当前版本）。
  <https://github.com/SWE-agent/mini-swe-agent>
- 批量入口：`mini-extra swebench --subset verified --split test --workers N`，输出
  `preds.json` + 每实例 trajectory；可直接喂官方 harness。
  <https://mini-swe-agent.com/v2/usage/swebench/>
- 适合作为同模型、同数据集的 **minimal scaffold baseline**；比较时不能只比分数，还要比
  max turns / cost / wall time / 是否 best-of-N。
- 有 `MSWEA_GLOBAL_COST_LIMIT` / call limit 等全局停止条件，可作为 M1 的参照。

### 2.3 Terminal-Bench 与 Harbor

- Terminal-Bench 2.0：89 tasks，ICLR 2026 论文；任务 = instruction + Docker environment +
  verifier + oracle solution。
  <https://www.tbench.ai/news/announcement-2-0>
  <https://proceedings.iclr.cc/paper_files/paper/2026/hash/444a3737adaee10d86ad2ef5f74468e6-Abstract-Conference.html>
- 当前官方版本是 **Terminal-Bench 4.0（66 tasks，11 个多容器任务、3 个需 H100）**；
  固定上游数据集 **4.0.0**（内容寻址任务版本）；AA Coding Agent Index v2.0 也用它。
  <https://github.com/modelscope/evalscope/blob/main/docs/zh/benchmarks/terminal_bench_v4.md>
- Harbor 是 Terminal-Bench 官方 harness：`harbor run -d terminal-bench/terminal-bench@4.0.0 --agent ...`；
  支持本地 Docker 与 Daytona/Modal 等云环境。
  <https://github.com/laude-institute/harbor>
  <https://pypi.org/project/harbor/>
- Harbor task 目录：
  `instruction.md`、`task.toml`、`environment/Dockerfile`、`tests/test.sh`、`solution/solve.sh`。
  verifier 必须写 `/logs/verifier/reward.txt` 或 `reward.json`。
  <https://www.harborframework.com/docs/tasks>
- Harbor 自定义 agent：优先 `BaseInstalledAgent`（把 agent CLI 装进任务容器，headless 运行）；
  `BaseAgent` 用于 agent loop 在容器外、经 `BaseEnvironment` 控制容器的形态。
  适配器实现 `install()` / `run(instruction, environment, context)` /
  `populate_context_post_run()`；`configure()` 每次 trial 都跑，适合上传 binary、写凭据、注册 MCP。
  <https://docs.harborframework.com/agents/custom-agents>
- Harbor 的 reward 是 task verifier 写的数值；多维度用 `reward.json`。
  <https://www.harborframework.com/docs/tasks>
- Docker 官方博客指出 agent eval 需要「框架 + 隔离环境」两层，并点名 reward hacking
  （改测试、改记分文件）；Docker Sandboxes 用 microVM 做更强隔离。
  <https://www.docker.com/blog/harbor-docker-sandboxes-run-safe-reproducible-agent-evals/>

### 2.4 Artificial Analysis

- Coding Agent Index v2.0 = FrontierCode v1.1（100 tasks）+ Terminal-Bench 4.0（66 tasks）+
  SWE-Atlas-QnA-AA（124 tasks），等权平均；每任务 3 次尝试。
  <https://artificialanalysis.ai/methodology/coding-agents-benchmarking>
- FrontierCode 是私有任务集，用「维护者是否愿意合并」的 rubric 评分；AA 用统一 harness（Stirrup）
  跑所有模型，因此是 **model + harness** 的组合分，不是裸模型分。
- Intelligence Index v5 的 Coding 25% = FrontierCode 2.0（15%）+ Terminal-Bench 4.0（10%）。
  <https://artificialanalysis.ai/articles/artificial-analysis-intelligence-index-v5>
- 结论：AA 只能做选模参考 / 外部对照；不能把 AA 分数写成 xylitol 已出分。若要对照，必须注明
  「同模型、不同 scaffold」。

## 3. xylitol 现状与缺口

### 已有

- `xylitol print`：非交互 headless；stdout 增量正文、stderr thinking、工具摘要；
  `XyEvent::Error` → 非 0 exit；工具单次失败不必然非 0。
  `llmanspec/specs/cli-print/cli-print.feature`
- `session.max_turns`：正整数 N，经 `should_stop_after_turn` 在 TurnEnd 后停止；
  缺省不装钩子（开放结束）。
  `llmanspec/specs/agent-runtime/agent-runtime.feature` r?（max_turns 条）
- `--trust` / `--no-trust`、`--config`、`--session`、`--model`、`--agent-profile`；
  print 不接受 `--attach` / `--port` / `--list-models`。
  `llmanspec/specs/cli-entry/cli-entry.feature`
- 每 run 新 session / 持久 session 由 bootstrap 控制；`ephemeral_sessions` 字段存在。
- 观测旁路：Langfuse / OTEL 可记 run 元数据，但不替代官方 grader。
- BDD/单元测试齐全，但**没有 benchmark eval harness**。

### 缺口

| 缺口 | 影响 | 归属 |
|---|---|---|
| 无 eval profile / 配置锁定 | 本地状态污染、不可复现 | M1 |
| 无墙钟超时 | 挂死实例拖垮整批 | M1 |
| 无成本/token 停止 | 单实例可能烧穿预算 | M1（runtime 能力） |
| 无 autosubmit 交卷协议 | 超限时没有 patch 可判 | M1 |
| print 下 ask/user 挂起策略未钉 | 可能卡住无人值守 run | M1 |
| 无 Docker 沙箱编排 | 不能跑 SWE-bench / Terminal-Bench | M0 |
| 无 Harbor adapter | 不能跑 Terminal-Bench | M3 |
| 无失败实例回归集 | 同一 bug 可复发 | M4 |
| 无结果报告规范 | 分数不可审计 / 不可比 | M0/M2 |

## 4. 最小底座设计

### 4.1 架构

```
eval CLI / Python driver
  ├─ dataset loader（固定 revision / instance ids）
  ├─ sandbox manager（Docker per instance）
  │    ├─ 拉/建官方 instance image
  │    ├─ 注入 xylitol binary + eval config
  │    ├─ 执行 `xylitol print ...`（带 timeout / max_turns）
  │    └─ 收集 `git diff --cached`
  ├─ predictions.jsonl
  ├─ official grader（swebench.harness.run_evaluation / Harbor）
  └─ report.json + logs + trajectories
```

推荐 M0 用 **Python orchestration**，理由：

- 官方 grader、SWE-bench 数据集、Harbor adapter 都是 Python 生态。
- 不在 Rust runtime 里引入 Docker/benchmark 依赖。
- Harbor M3 适配器本身是 Python `BaseInstalledAgent`，可复用同一 driver 经验。
- Rust 侧只提供稳定的 `xylitol` headless binary。

落点建议：

```
tools/eval/
  pyproject.toml
  src/xylitol_eval/
    cli.py          # run / grade / report
    datasets.py     # SWE-bench / Harbor task 选择
    sandbox.py      # docker run/exec/cp/timeout
    agent.py        # xylitol print 调用 + patch 收集
    grade.py        # official grader 调用 + results.json 解析
    report.py       # run metadata / resolved % / cost / wall time
  tests/
```

M0 也可先落 `scripts/lab_eval_swebench.py` 做一次性探针；一旦要重复使用/进 CI，再升到 `tools/eval/`。

### 4.2 M0 流程（SWE-bench Verified 5）

1. 选实例：固定 dataset revision + 显式 instance ids（不要随机后不记录）。
2. 准备镜像：优先 pull 官方/预构建 `sweb.eval.x86_64.<instance_id>`；缺失再 build。
3. 启动容器：`docker run -d --rm ... sleep infinity`；工作目录通常是 `/testbed`。
4. 注入：
   - `xylitol` Linux binary（推荐 musl static；或 CI 产出 glibc 兼容 binary）。
   - eval config（只含 model/provider、`session.max_turns`、工具闭集、无 MCP/hooks/OTEL）。
   - API key 经 env 注入；不要写入 config / 日志。
5. 运行：
   - `timeout --signal=TERM --kill-after=10s <wall> xylitol print --no-trust --config ... "<problem_statement>"`
   - `HOME` / `XYLITOL_CONFIG_DIR` / `XYLITOL_PROJECT_DIR` 指向容器内隔离目录。
   - 每实例新 session；禁止 resume 旧 session。
6. 收 patch：
   - `git add -A && git diff --cached`（包含新增文件；不要只 `git diff`）。
   - 若 patch 为空，仍写 prediction（官方 harness 会计 empty patch）。
7. 写 `preds.jsonl`：`instance_id` / `model_name_or_path` / `model_patch`。
8. 官方判分：
   `python -m swebench.harness.run_evaluation --dataset_name princeton-nlp/SWE-bench_Verified --predictions_path preds.jsonl --run_id <id> --max_workers N --cache_level instance`
9. 解析 `results.json`：resolved / unresolved / errors / empty patches；每实例 report。
10. 报告：run metadata + 结果 + 失败实例列表 + 日志路径。

验收：

- 5/5 实例都有 report（不是 5/5 resolved）。
- 同一命令、同一 binary/config/dataset revision 重跑，resolved 集合可复现（允许模型非确定性时注明）。
- 失败实例的 `run_instance.log` / `test_output.txt` 可定位。

### 4.3 产物与报告规范

建议每次 run 目录：

```
eval-runs/<run_id>/
  run.json              # xylitol version/commit, binary sha256, model, profile, dataset revision, harness version
  config.yaml           # eval profile（脱敏）
  preds.jsonl
  results.json          # 官方 grader 摘要
  logs/                 # 每实例 stdout/stderr/trajectory
  reports/              # 每实例 report.json / test_output.txt
  summary.md            # 人读报告
```

`run.json` 必填：

- xylitol git commit + binary sha256 + target triple
- model provider / model id / thinking level
- eval profile hash（max_turns / timeout / tools / permission / network）
- dataset name + revision + instance ids
- harness name + version（swebench / harbor）
- pass@1 / best-of-N 标注
- wall time / token usage / cost（若可得）
- 是否允许网络 / 是否过滤测试文件

报告禁止：

- 不写「xylitol 在 AA 上 X 分」。
- 不混入未标注的 best-of-N / 多次重试 / 人工挑选。
- 不把 `0 failed` 当成全成功。

### 4.4 隔离、防作弊与网络

- **每实例独立容器**，不要复用工作树；结束后删除容器/临时卷。
- **verifier 与 agent 分离**：agent 不应能改 `tests/` / 记分文件；SWE-bench 的 test patch 在判分阶段
  应用，若 model patch 改了测试文件，可能 apply 冲突或污染判分。M0 先记录，不做静默过滤；
  M2 可评估「判分前 reset tests」策略，但必须在报告里注明。
- **网络**：SWE-bench 允许 agent 联网会带来「查上游 PR」污染。M0 至少记录网络策略；
  理想是 egress allowlist 只放 model API，或本地 provider 无外网。Harbor/Docker Sandboxes 提供
  更强隔离与网络策略，M3 再采用。
- **Docker 不是强安全边界**：本地个人机器可接受，但恶意 benchmark agent 可能逃逸/读宿主；
  需要更强隔离时用 microVM / 云沙箱。
- **凭据**：API key 只经 env / Harbor secret 注入；不写入 config、不写日志；teardown 清理。

### 4.5 eval profile 与停止条件

M1 目标 profile（建议 YAML）：

```yaml
eval:
  name: swebench-verified-mini
  max_wall_seconds: 1800
  max_turns: 80
  max_cost_usd: 1.50
  max_tokens: null
  autosubmit_on_limit: true
  network: model_only
  tools: [read, bash, edit, write, grep, find, ls]
  permission:
    filesystem:
      write_allowed: ["/testbed/**"]
  ask: disabled
```

现状：

- `session.max_turns` 已有。
- `max_wall_seconds` 可由外部 `timeout` 包装（M0 即可）。
- `max_cost_usd` / `max_tokens` 需要 runtime 在每 TurnEnd 后读累计 usage 并触发 stop hook；
  这是 agent-runtime 行为变更，应单独 SDD change。
- `autosubmit_on_limit`：超限时仍把当前 `git diff` 交卷，而不是丢弃；M1 需要定义「超限后是否
  允许再跑一次收尾命令」。
- `ask: disabled`：print 无 UI；确保 ask 工具不在工具闭集、MCP elicitation 不挂起。

### 4.6 Harbor adapter（M3）

```python
from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext

class XylitolAgent(BaseInstalledAgent):
    async def install(self, environment: BaseEnvironment) -> None:
        await environment.upload_file(local_binary, "/usr/local/bin/xylitol")
        await self.exec_as_root(environment, command="chmod 755 /usr/local/bin/xylitol")

    async def configure(self, environment: BaseEnvironment) -> None:
        # 每次 trial 都跑：写 eval config / API key / 清理旧 session
        ...

    @with_prompt_template
    async def run(self, instruction: str, environment: BaseEnvironment, context: AgentContext) -> None:
        await self.exec_as_agent(
            environment,
            command=f"timeout 1800 xylitol print --no-trust --config /eval/config.yaml {shlex.quote(instruction)}",
        )

    def populate_context_post_run(self, context: AgentContext) -> None:
        # 解析 token usage / cost / trajectory
        ...
```

运行：

```bash
harbor run -d terminal-bench/terminal-bench@4.0.0 \
  --agent-import-path xylitol_eval.harbor:XylitolAgent \
  -m <model> --n-concurrent 4
```

注意：

- Harbor 的 `-a` 已知 agent 名优先于 `--agent-import-path`；自定义 adapter 不要同时传 `-a`。
- `install()` 在 `--ak skip_install=true` 时跳过；必须每次 trial 做的事放 `configure()`。
- Harbor reward 由 task verifier 写；adapter 不自己判分。
- Terminal-Bench 4.0 当前 66 tasks；M3 先跑 10–20 tasks。
- Harbor 版本约束（evalscope 实测）：`harbor>=0.14.0,<1.0.0`，Python `>=3.12`；
  完整 4.0 需 H100 与多容器 Docker，或用 Modal 等云环境。

## 5. 分阶段与工作量

| 阶段 | 交付 | 估算 | 验收 |
|---|---|---|---|
| M0 | Python driver + Docker + Verified 5；preds + official results | 1–2 天 | 5/5 grade；报告可复现 |
| M1 | eval profile / timeout / max_turns / config lock / no-ask；cost/token 报告 | 2–4 天 | 超限可停、可交卷；profile hash 记录 |
| M2 | Verified ~50；mini-SWE-agent 同口径 baseline；JSON/Markdown 报告 | 3–5 天 | 同模型同数据对比；失败可定位 |
| M3 | Harbor `BaseInstalledAgent` + Terminal-Bench 10–20 | 1–2 周 | reward 来自 task verifier；可本地/云跑 |
| M4 | 失败实例 → 回归集；小集 CI 旁路 | 3–5 天 | 回归集稳定、不阻塞主 CI |
| M5 | 全量 / 云并行 / leaderboard | 后置 | 只在 M2 稳定后 |

## 6. ROI 与建议

- **M0 ROI 最高**：没有它，所有「模型/prompt 改动是否进步」都是主观判断。
- **M1 ROI 高**：没有停止条件，M0 只能跑玩具集；成本/墙钟不可控会阻止 M2。
- **M2 ROI 中高**：50 实例才有统计意义；mini baseline 让分数可解释。
- **M3 ROI 中**：Terminal-Bench 更贴近 xylitol 的终端 agent 定位，但 Harbor adapter + 镜像 +
  云调度成本高；等 M2 稳定后再做。
- **M4 ROI 中**：回归集是长期质量杠杆，但依赖 M0/M2 的失败数据。
- **M5 ROI 低（当前）**：全量/云跑在没有稳定底座前只会放大噪声。

建议下一批只做 **M0 + M1**，并把 M2 作为紧随其后的第二个 change。M3 单独提案，不塞进 M0。

## 7. 开放问题

1. SWE-bench 子集：Verified 5 → 50，还是 Lite 300？Verified 更干净但 500 全量成本高；
   Lite 更便宜但噪声/污染更高。建议 Verified 固定 50 子集，记录 instance ids。
2. baseline：mini-SWE-agent 同模型同 max_turns；是否允许 best-of-N？默认 pass@1，best-of-N 必须标注。
3. binary 分发：musl static 还是 glibc？官方 SWE-bench 镜像的 glibc 版本需实测。
4. 网络策略：M0 是否允许外网？若允许，分数只能当上限；M2 前应至少记录。
5. 测试文件改动：判分前 reset tests 还是让官方 harness 自然失败？建议先记录，不静默过滤。
6. 成本停止：runtime 新增 cost/token stop hook，还是 eval driver 外部轮询？外部轮询无法阻止已发出的请求；
   建议 runtime hook，单独 SDD。
7. 结果落点：`eval-runs/` 本地 / CI artifact / Langfuse Dataset；Langfuse 只作旁路，不作分数。

## 8. 引用

- SWE-bench evaluation guide: <https://github.com/swe-bench/swe-bench/blob/main/docs/guides/evaluation.md>
- SWE-bench dataset counts: <https://github.com/swe-bench/swe-bench/blob/02e7a74ffd0b707aab73d203fe87bdc7c76afc8e/docs/assets/evaluation.md>
- SWE-bench CLI: <https://www.swebench.com/SWE-bench/reference/cli/>
- Epoch AI SWE-bench image registry: <https://github.com/epoch-research/SWE-bench>
- mini-SWE-agent: <https://github.com/SWE-agent/mini-swe-agent>
- mini-SWE-agent SWE-bench: <https://mini-swe-agent.com/v2/usage/swebench/>
- Terminal-Bench 2.0 announcement: <https://www.tbench.ai/news/announcement-2-0>
- Terminal-Bench 2.0 paper: <https://proceedings.iclr.cc/paper_files/paper/2026/hash/444a3737adaee10d86ad2ef5f74468e6-Abstract-Conference.html>
- Harbor: <https://github.com/laude-institute/harbor>
- Harbor custom agents: <https://docs.harborframework.com/agents/custom-agents>
- Harbor task format: <https://www.harborframework.com/docs/tasks>
- Harbor rewards: <https://github.com/huggingface/repo2rlenv/blob/27d04ee8203cd5b2f20c3997d13836e5f5b64238/docs/concepts/rewards.mdx>
- Terminal-Bench 4.0 (evalscope): <https://github.com/modelscope/evalscope/blob/main/docs/zh/benchmarks/terminal_bench_v4.md>
- Terminal-Bench v4.0.0 release: <https://zenodo.org/records/22105681>
- Docker + Harbor sandboxes: <https://www.docker.com/blog/harbor-docker-sandboxes-run-safe-reproducible-agent-evals/>
- Artificial Analysis Coding Agent Index methodology: <https://artificialanalysis.ai/methodology/coding-agents-benchmarking>
- Artificial Analysis Intelligence Index v5: <https://artificialanalysis.ai/articles/artificial-analysis-intelligence-index-v5>

## 9. M0 命令草图

```bash
# 1) 准备 5 个固定 instance（按 repo 分层抽样，ids 写入 run.json）
# 2) 拉官方/预构建镜像
docker pull swebench/sweb.eval.x86_64.<instance_id>:latest

# 3) 启动隔离容器
docker run -d --rm --name "xy-eval-<instance_id>" \
  swebench/sweb.eval.x86_64.<instance_id>:latest sleep infinity

# 4) 注入 binary + eval config
docker cp ./xylitol "xy-eval-<instance_id>:/usr/local/bin/xylitol"
docker cp ./eval-config.yaml "xy-eval-<instance_id>:/eval/config.yaml"

# 5) headless 运行（容器内）
docker exec "xy-eval-<instance_id>" \
  timeout --signal=TERM --kill-after=10s 1800 \
  env HOME=/eval/home XYLITOL_CONFIG_DIR=/eval/config XYLITOL_PROJECT_DIR=/testbed \
  /usr/local/bin/xylitol print --no-trust --config /eval/config.yaml \
  "<problem_statement>"

# 6) 收 patch（包含新增文件）
docker exec "xy-eval-<instance_id>" sh -lc \
  'cd /testbed && git add -A && git diff --cached' > "patches/<instance_id>.diff"

# 7) 写 preds.jsonl，交给官方 grader
python -m swebench.harness.run_evaluation \
  --dataset_name princeton-nlp/SWE-bench_Verified \
  --predictions_path preds.jsonl \
  --run_id "xy-$(date +%Y%m%d-%H%M%S)" \
  --max_workers 2 \
  --cache_level instance
```

Python driver 只负责编排这些步骤；**不要**在 driver 里实现测试解析/判分。

## 10. 风险表

| 风险 | 影响 | 缓解 |
|---|---|---|
| 上游污染 / 查 PR | 分数虚高 | 网络 allowlist / 本地 provider / 记录网络策略 |
| 改测试或记分文件 | reward hacking | agent/verifier 分离；判分前 reset tests；审计 patch |
| 镜像/架构不一致 | 分数不可比 | 固定 image digest + arch；记录 `docker inspect` |
| 模型非确定性 | 重跑分数漂移 | temperature=0 / seed；pass@1 与 best-of-N 分开报告 |
| 成本/墙钟失控 | 单实例烧预算 | max_turns + wall timeout + cost/token stop |
| 空 patch / patch apply 失败 | 被误读为模型失败 | 区分 empty / apply error / test fail / infra error |
| 本地状态污染 | 不可复现 | 临时 HOME / config / session；eval profile hash |
| Docker 逃逸 | 本机风险 | 本地接受；更强隔离用 microVM / 云沙箱 |

## 11. 建议的 SDD 切片

- `c3006-add-minimal-eval-harness`（M0）：Python driver + Docker + Verified 5 + official grader。
  `needs_specs_change: false` 或只加 `test-qa-gate`/`cli-print` 的评测运行约束？M0 是开发工具，
  不改变产品行为；建议 `needs_specs_change: false`，用单测/集成测试守护。
- 后续 `c3007-update-eval-profile`（M1）：eval profile / timeout / cost stop / autosubmit。
  若新增 runtime stop hook，属行为合约变更，需 specs（`agent-runtime`）。
- `c3008-add-terminal-bench-harbor`（M3）：Harbor adapter，独立提案。

M0 不建议塞进 `just qa` 默认门禁；可作为 `just qa-eval-smoke` 手动/夜间入口，或 CI 非阻塞 job。
