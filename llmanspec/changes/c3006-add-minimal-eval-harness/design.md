# Design：最小 Agent Eval 底座

## 目标形态

```
eval CLI / Python driver
  ├─ dataset loader（固定 revision / instance ids）
  ├─ sandbox manager（Docker per instance）
  │    ├─ 拉/建官方 instance image
  │    ├─ 注入 xylitol binary + eval config
  │    ├─ 执行 `xylitol print ...`（带 timeout / max_turns）
  │    └─ 收集 `git add -A && git diff --cached`
  ├─ predictions.jsonl
  ├─ official grader（swebench.harness.run_evaluation / Harbor）
  └─ report.json + logs + trajectories
```

## 决策

### 1. 为什么是 Python driver

- 官方 grader、SWE-bench 数据集、Harbor adapter 都是 Python 生态。
- 不在 Rust runtime 里引入 Docker/benchmark 依赖。
- Rust 侧只提供稳定的 `xylitol` headless binary；评测编排独立演进。

### 2. M0 流程（Verified 5）

1. 选实例：固定 dataset revision + 显式 instance ids（写入 `run.json`）。
2. 准备镜像：优先 pull 官方/预构建 `sweb.eval.x86_64.<instance_id>`；缺失再 build。
3. 启动容器：`docker run -d --rm ... sleep infinity`；工作目录通常是 `/testbed`。
4. 注入：binary、eval config、API key（仅 env）。
5. 运行：`timeout ... xylitol print --no-trust --config ... "<problem_statement>"`；
   每实例新 session，`HOME` / `XYLITOL_CONFIG_DIR` / `XYLITOL_PROJECT_DIR` 指向隔离目录。
6. 收 patch：`git add -A && git diff --cached`（含新增文件）。
7. 写 `preds.jsonl`；空 patch 也写（官方 harness 计 empty patch）。
8. 官方判分：`python -m swebench.harness.run_evaluation --dataset_name princeton-nlp/SWE-bench_Verified --predictions_path preds.jsonl --run_id <id> --max_workers N --cache_level instance`。
9. 解析 `results.json`：resolved / unresolved / errors / empty patches。
10. 报告：run metadata + 结果 + 失败实例列表 + 日志路径。

### 3. 产物规范

```
eval-runs/<run_id>/
  run.json              # xylitol commit / binary sha256 / model / profile / dataset revision / harness version
  config.yaml           # eval profile（脱敏）
  preds.jsonl
  results.json          # 官方 grader 摘要
  logs/                 # 每实例 stdout/stderr/trajectory
  reports/              # 每实例 report.json / test_output.txt
  summary.md            # 人读报告
```

报告禁止：把 AA 分数当 xylitol 分数；混入未标注的 best-of-N；把 `0 failed` 当成全成功。

### 4. 隔离与防作弊

- 每实例独立容器；结束后删除容器/临时卷。
- agent 与 verifier 分离；判分前是否 reset tests 必须记录，不做静默过滤。
- 网络策略至少记录；理想 egress allowlist 只放 model API。
- Docker 不是强安全边界；更强隔离用 microVM / 云沙箱（M3 再采用）。
- API key 只经 env 注入；teardown 清理；不写 config / 日志。

### 5. eval profile（M1）

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

现状：`session.max_turns` 已有；墙钟超时可由外部 `timeout` 包装；cost/token stop 与
autosubmit 属 runtime 行为变更，M1 单独 SDD。

### 6. Harbor adapter（M3，独立 change）

- 继承 `BaseInstalledAgent`；`install()` 上传 binary，`configure()` 每次 trial 写 config/凭据，
  `run()` 执行 headless print，`populate_context_post_run()` 解析 usage。
- reward 由 task verifier 写 `/logs/verifier/reward.txt|json`；adapter 不自己判分。
- Terminal-Bench 当前 4.0（66 tasks）；先跑 10–20 tasks。

## 分阶段

| 阶段 | 交付 | 验收 |
|---|---|---|
| M0 | Python driver + Docker + Verified 5 + official grader | 5/5 grade；报告可复现 |
| M1 | eval profile / timeout / max_turns / no-ask / config lock | 超限可停、可交卷；profile hash 记录 |
| M2 | Verified 50 + mini-SWE-agent baseline + 报告 | 同模型同数据对比；失败可定位 |
| M3 | Harbor `BaseInstalledAgent` + Terminal-Bench 10–20 | reward 来自 task verifier |

M0 不建议进 `just qa` 默认门禁；可作手动/夜间 `lab_` 探针或非阻塞 CI job。

## 测试边界

- driver 纯函数：dataset 选择、patch 收集、preds 序列化、results 解析（单测，不碰 Docker）。
- Docker 编排：`lab_` 一次性探针 + 手动 smoke；不进默认 `just qa`。
- 报告 schema：单测锁定必填字段。

## 非目标

- 不做多用户评测平台 / leaderboard。
- 不用 TUI 交互或自动按键跑 benchmark。
- 不用 Langfuse judge 当主分数。
- 不在 M0 做全量并行 / 云调度。
