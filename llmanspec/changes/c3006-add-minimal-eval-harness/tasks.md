# Tasks

## Phase 0：骨架与 dry-run

- [ ] 0.1 建 `tools/eval/` 包骨架：`pyproject.toml`、`uv.lock`、`README.md`、`src/xylitol_eval/`
- [ ] 0.2 `ports.py`：DatasetSource / Sandbox / AgentRunner / Grader / Reporter
- [ ] 0.3 `config.py`：eval.yaml 加载、校验、phase 枚举、sha256
- [ ] 0.4 `report.py`：run.json / summary.md schema（预留 reward/regression/cloud 可选字段）
- [ ] 0.5 `cli.py`：dry-run / run / grade / report / doctor；未实现阶段明确报错
- [ ] 0.6 `README.md`：工具契约、密钥占位、使用说明、非目标
- [ ] 0.7 `examples/`：mini / 50 / terminal-bench 配置样例（无真实 key）
- [ ] 0.8 单测骨架：config / report / cli dry-run
- [ ] 0.9 `.github/workflows/eval-bin.yaml`：仅 workflow_dispatch，ubuntu-22.04，glibc 构建
- [ ] 0.10 `just`：eval-test / eval-dryrun / eval-smoke（非默认 qa）
- [ ] 0.11 `.gitignore`：eval 运行目录 / 本地 binary / venv
- [ ] 0.12 验收：dry-run 输出正确；unit tests 绿；手动触发 CI 产出 binary
- [ ] 0.13 本地 Docker 构建 Linux binary（glibc 2.35）：记录 sha256 / target triple
- [ ] 0.14 magpie catalog 读取：alias 用 catalog id，生成临时 xylitol config（env key）
- [ ] 0.15 成本护栏：按 catalog 价格估算 5/50 实例预算；超限告警

## Phase 1：M0 冒烟

- [ ] 1.1 `dataset.py`：固定 revision + 显式 ids + limit；本地缓存
- [ ] 1.2 `sandbox.py`：Docker pull/run/exec/cp/rm；每实例独立容器与清理
- [ ] 1.3 `agent.py`：注入 binary/config；隔离 HOME / 配置目录；`--no-trust`；timeout；模型 key 只经 env
- [ ] 1.4 `patch.py`：`git add -A && git diff --cached`；空 patch 也写 prediction
- [ ] 1.5 `preds.py`：`instance_id` / `model_name_or_path` / `model_patch`
- [ ] 1.6 `grade.py`：调 `run_evaluation`；解析 schema v2；失败分类
- [ ] 1.7 `report.py`：run.json / summary.md；记录 binary hash、config hash、dataset revision、harness 版本
- [ ] 1.8 单测：dataset / patch / preds / grade / report
- [ ] 1.9 fake provider 端到端：容器编排与日志收集
- [ ] 1.10 Verified 5 dry-run：检查 ids、镜像、命令、config hash
- [ ] 1.11 Verified 5 实跑：5/5 被处理；error=0；报告可复现
- [ ] 1.12 验收：`completed + empty_patch = submitted`；每实例日志齐全
- [ ] 1.13 usage 记录：从 session JSONL 汇总 tokens / cost

## Phase 2：M1 eval profile

- [ ] 2.1 墙钟：driver 对 exec 设 deadline；kill 后仍收 patch
- [ ] 2.2 max_turns：config `session.max_turns`；确认 exit 0
- [ ] 2.3 配置锁：每实例临时 HOME；断言无项目 `.xylitol/config.yaml`；记录 config sha256
- [ ] 2.4 无 ask 断言：确认 print 不装配 ask 工具
- [ ] 2.5 不 resume：不传 `--session`；每实例新 session
- [ ] 2.6 验收：timeout 仍收 patch；config hash 稳定；max_turns exit 0
- [ ] 2.7 预算提示：记录成本；超限提示（不实现 runtime cost stop）

## Phase 3：M2 Verified 50

- [ ] 3.1 扩到 50 实例；逐实例镜像清理或扩容
- [ ] 3.2 `max_workers` 1–2；避免 Docker VM OOM
- [ ] 3.3 mini-SWE-agent baseline：同模型、同数据、同停止条件
- [ ] 3.4 对比报告：resolved%、cost、wall time、turns
- [ ] 3.5 验收：同口径对比表；失败实例可定位

## Phase 4：M3 Harbor

- [ ] 4.1 `BaseInstalledAgent`：install / configure / run / populate_context_post_run
- [ ] 4.2 Terminal-Bench 4.0 先跑 10–20 任务
- [ ] 4.3 reward 只来自 task verifier 的 `/logs/verifier/reward.txt|json`
- [ ] 4.4 验收：reward 文件生成；与 Harbor 口径一致

## Phase 5：M4 / M5

- [ ] 5.1 失败实例 → Langfuse Dataset（旁路）
- [ ] 5.2 小集回归进非阻塞 CI
- [ ] 5.3 云并行全量 / leaderboard 提交
- [ ] 5.4 验收：回归可跑；全量可复现

## 收口

- [ ] 6.1 `llman-sdd validate --strict` 结构通过
- [ ] 6.2 `just qa` / `just eval-test` 绿
- [ ] 6.3 确认后再开 PR；确认前不 push
