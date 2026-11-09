# Tasks

## Phase 0：骨架与 dry-run（已完成）

- [x] 0.1 `tools/eval/` 包骨架：`pyproject.toml`、`uv.lock`、`README.md`、`src/xylitol_eval/`
- [x] 0.2 `ports.py`：DatasetSource / Sandbox / AgentRunner / Grader / Reporter
- [x] 0.3 `config.py`：eval.yaml 加载、校验、phase 枚举、sha256
- [x] 0.4 `report.py`：run.json / summary.md schema
- [x] 0.5 `cli.py`：dry-run / run / grade / report / doctor
- [x] 0.6 `README.md`：工具契约、密钥占位、使用说明、非目标
- [x] 0.7 `examples/`：50 / terminal-bench 配置样例（无真实 key）
- [x] 0.8 单测骨架：config / report / cli dry-run
- [x] 0.9 `.github/workflows/eval-bin.yaml`：仅 workflow_dispatch，ubuntu-22.04，static musl 构建
- [x] 0.10 `just`：eval-test / eval-smoke / eval-typical
- [x] 0.11 `.gitignore`：eval 运行目录 / 本地 binary / venv / swebench logs
- [x] 0.12 本地 Docker 构建 Linux binary（static musl）：记录 sha256 / target triple
- [x] 0.13 magpie catalog 读取：alias 用 catalog id，生成临时 xylitol config（env key）
- [x] 0.14 fake provider 端到端：容器编排与日志收集
- [x] 0.15 固定模型 `group/glm-5.3-flash`；同一轮对比中 xylitol/Pi 同模型同配置

## Phase 1：Harbor 跑通 xylitol（已完成）

- [x] 1.1 安装 Harbor（0.25）到临时 venv；确认 `harbor run` 可用
- [x] 1.2 新增 `tools/eval/harbor/xylitol_agent.py`：`BaseInstalledAgent`
- [x] 1.3 install 上传 binary；configure 写临时 config；run 执行 `xylitol print`
- [x] 1.4 key 写入容器 0600 `secret.env`；不把 key 放进 Harbor `exec -e` argv
- [x] 1.5 Harbor + xylitol 单 task 跑通：agent 执行，verifier 产出 reward
- [x] 1.6 记录 job 路径、agent log、verifier reward、失败分类
- [x] 1.7 Docker Hub 不可达的本地 workaround：`DOCKER_BUILDKIT=0`
- [x] 1.8 更新 issue #21：决策、Phase 1 状态、下一阶段

## Phase 2：基准单 task（已完成）

- [x] 2.1 static musl binary（`x86_64-unknown-linux-musl`）：statically linked，sha256 记录
- [x] 2.2 Terminal-Bench 4.0 单 task（`layout-config-recreation`，单容器/CPU）：reward 0.0，verifier 跑通
- [x] 2.3 DeepSWE 单 task Harbor spike（`httpx-multipart-response-parsing`）：reward 0.0，verifier 跑通
- [x] 2.4 model/config lock：`tools/eval/model-lock.yaml` + `doctor --lock`
- [x] 2.5 验收：TB4.0 / DeepSWE 各 1 task 可复现；smoke 不要求通过

## Phase 3：xylitol vs Pi（已完成）

- [x] 3.1 Pi 单 task：`-a pi` + magpie endpoint + `model_api=openai-responses`
- [x] 3.2 Pi baseline 用预装镜像，同数据同 verifier；mini-SWE 已移除
- [x] 3.3 ~~SWE Verified 25 × N=3~~（已移除：镜像仅 linux/amd64，arm64 主机 Compose build 失败）
- [x] 3.4 TB4.0 10 × N=3：移出当前典型集；只保留 xylitol N=2 历史记录
- [x] 3.5 DeepSWE 5-task typical × N=3：15 trials，xylitol 7/15，Pi 6/15
- [x] 3.6 报告 pass@1、flip rate、cross_scaffold_flip、cost、wall、turns
- [x] 3.7 验收：每个 scaffold/benchmark 单独一行；不混分
- [x] 3.8 Phase 3 pilot：SWE astropy 1 task × N=3 × 3 scaffolds（历史；该基准已移除）
- [x] 3.9 baseline：DeepSWE 5-task × N=3 × xylitol/Pi；全量 10 task 留待以后
- [x] 3.10 下一次对比：同命令重跑，比较 pass@1 / flip / cross-scaffold

## 后续（不在本 change）

- Phase 4：3–5 个 golden fixture + expected patch/test。
- Phase 4：mock/replay OpenAI-compatible server（回放 session JSONL）。
- Phase 4：session snapshot 归一化对比。
- Phase 4：非阻塞 PR 门禁；失败实例可提升为 golden。
- Phase 5：Harbor 小集回归进非阻塞 CI。
- Phase 5：失败实例 → Langfuse Dataset（旁路）。
- Phase 5：云并行全量 / leaderboard 提交。
- 全量 DeepSWE 10 task、TB4 10 task。

## 收口

- [x] 6.1 `llman-sdd validate --strict` 结构通过
- [x] 6.2 `just eval-test` 绿；CI 已成功
- [x] 6.3 代码已 push 到 `main`；未开 PR
