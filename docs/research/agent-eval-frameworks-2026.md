# Coding Agent Eval 框架调研（2025–2026）

> 面向 xylitol roadmap：可重复端到端回归、优先复用 OTEL + Langfuse traces/scores。
> 一手来源：各框架官方 docs / GitHub README / 官方 blog（2025–2026 活跃项）。

## 1. 一句话结论

**以 Langfuse Datasets + Experiments + Scores/LLM-as-judge 为 eval 主拼图（直接吃现有 OTEL traces），CI 用 `langfuse/experiment-action` 做回归闸；外部对标用 Harbor 跑 SWE-bench Verified / Terminal-Bench 2.x；Promptfoo 仅作可选安全/trajectory 断言层，不把 Braintrust/Inspect UI 当第二观测栈。**

---

## 2. 对比表

| 框架 | 定位 | coding-agent 适配度 | 与 Langfuse/OTEL 关系 | CI 友好度 | 许可/自托管 | 一手来源 |
|------|------|---------------------|----------------------|-----------|-------------|----------|
| **Langfuse**（Datasets / Experiments / Scores / LLM-as-judge） | 观测 + eval 闭环：trace → dataset → experiment → score | ★★★★★ 回归集、在线/离线 eval、代码 evaluator | **原生 OTEL backend**；SDK 基于 OTEL；scores 可挂 trace/observation/experiment | ★★★★★ `experiment-action` + `RegressionError` 阈值闸 | MIT；[自托管](https://langfuse.com/docs/observability/get-started) | [Overview](https://langfuse.com/docs/evaluation/overview) · [OTEL](https://langfuse.com/integrations/native/opentelemetry) · [Experiments CI/CD](https://langfuse.com/docs/evaluation/experiments/experiments-ci-cd) |
| **Promptfoo** | 声明式 eval + agent red team + trajectory 断言 | ★★★★ agent provider 钩子、`trajectory:*` 断言 | 内置 OTLP receiver；可转发到 Jaeger/Tempo；与 Langfuse **并行**非原生合并 | ★★★★★ `promptfoo-action`、redteam CI | MIT | [GitHub](https://github.com/promptfoo/promptfoo) · [Tracing](https://www.promptfoo.dev/docs/tracing/) · [CI/CD](https://www.promptfoo.dev/docs/integrations/ci-cd/) |
| **Braintrust** | Eval 平台：dataset + scorer + 在线评分 + CI | ★★★ 通用 agent task；autoevals **不评整条 trace** | 独立 tracing 栈；与 Langfuse 无官方一体集成 | ★★★★ Eval SDK + CI 文档 | 开源 core + SaaS | [Evaluate](https://www.braintrust.dev/docs/evaluate) · [Autoevals](https://www.braintrust.dev/docs/evaluate/autoevals) |
| **OpenAI Evals**（开源 + Platform API） | LLM eval registry + hosted graders | ★★ coding 需自定义 completion fn | 无 Langfuse/OTEL 一体方案 | ★★★ API/CLI | 开源 MIT；Platform 将关停 | [GitHub](https://github.com/openai/evals) · [API 弃用说明](https://developers.openai.com/api/docs/guides/evals) |
| **Anthropic**（工程指南，非 harness） | agent eval 方法论：capability vs regression、三种 grader | ★★★★★ coding agent 最佳实践 SSOT | 点名 Harbor / Braintrust / **Langfuse** 为可组合框架 | —（指南） | — | [Demystifying evals](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents) |
| **Inspect AI + Inspect Evals** | 英方 AISI 通用 LLM/agent eval 框架 + 社区 benchmark 包 | ★★★★ SWE-bench/GAIA 等；自带 View UI | 自有 log 格式；非 Langfuse 原生 | ★★★ CLI/API、`--json` 后台跑 | MIT | [Inspect](https://inspect.aisi.org.uk/) · [Inspect Evals 公告](https://www.aisi.gov.uk/blog/inspect-evals) |
| **DeepEval** | Pytest 式 LLM/agent 单测 + trace 级 metric | ★★★★ agent metrics（TaskCompletion、ToolCorrectness 等） | **可消费 OTEL spans**；结果需自行 `score` 回 Langfuse | ★★★★★ 设计目标即 CI/pytest | MIT | [Docs](https://deepeval.com/docs/introduction) · [Agent evals](https://deepeval.com/docs/getting-started-agents) · [OTEL tracing](https://deepeval.com/docs/evaluation-llm-tracing) |
| **Ragas** | RAG/agent metric 库 + CLI quickstart | ★★★ ToolCallAccuracy / AgentGoalAccuracy 等 | 无 Langfuse 官方一体；偏 Python 侧离线 | ★★★ CLI `agent_evals` 模板 | Apache-2.0 | [Agent metrics](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/agents/) · [Agent evals quickstart](https://docs.ragas.io/en/v0.4.3/howtos/cli/agent_evals/) |
| **SWE-bench / Verified + harness** | 真实 GitHub issue → patch → 测试闸 | ★★★★★ coding agent 金标准 | 独立 Docker harness；与 Langfuse **正交**（后处理写 score） | ★★★ 重（Docker/并行）；有 Modal/sb-cli | MIT | [GitHub](https://github.com/SWE-bench/SWE-bench/) · [Verified](https://www.swebench.com/verified) · [Harness](https://www.swebench.com/SWE-bench/reference/harness/) |
| **mini-SWE-agent** | 极简 bash ReAct + SWE-bench 批跑脚本 | ★★★★★ LM 对标基线（非 xylitol 本体） | 产出 `preds.jsonl` → 官方 harness | ★★★ `mini-extra swebench` 批处理 | MIT | [GitHub](https://github.com/SWE-agent/mini-swe-agent) · [SWE-bench 用法](https://mini-swe-agent.com/latest/usage/swebench/) |
| **Harbor + Terminal-Bench** | 容器化 agent eval 运行时 + benchmark registry | ★★★★★ 终端/工程长任务；TB 2.x 官方 harness | 自有 job viewer；可用 Langfuse **外挂**记 run-level score | ★★★★ `harbor run` + `--upload`；云 sandbox 并行 | Apache-2.0 | [Harbor](https://github.com/harbor-framework/harbor) · [Terminal-Bench](https://www.tbench.ai/) · [TB 2.1](https://github.com/harbor-framework/terminal-bench-2-1) |
| **LiveCodeBench** | 竞赛题代码生成/修复（非 repo agent） | ★★ 测模型代码能力，非 harness 端到端 | 无 | ★★★ CLI runner | MIT | [GitHub](https://github.com/livecodebench/livecodebench) |
| **GAIA**（Inspect Evals） | 通用助手：浏览/bash 工具 | ★★ 偏 research assistant，非 coding harness | 经 Inspect，非 Langfuse | ★★ Docker 依赖 | MIT | [Inspect Evals GAIA](https://ukgovernmentbeis.github.io/inspect_evals/evals/assistants/gaia/) |
| **AgentBench**（THUDM） | 8 环境通用 agent（OS/DB/Web…） | ★★ OS 子任务相关；整体偏通用 | 独立栈 | ★★ 容器化 FC 版 | Apache-2.0 | [GitHub](https://github.com/THUDM/AgentBench/) |
| **WebArena** | 自托管 Web 导航 | ★ 非 coding；作者推荐 [TheAgentCompany](https://the-agent-company.com) 做终端/编码 | 无 | ★★ 自托管重 | Apache-2.0 | [GitHub](https://github.com/web-arena-x/webarena) |
| **Arena-Hard** | Chatbot Arena 近似：LLM judge 比答案 | ★ 含 SE 开放题，但**非 agent harness** | 无 | ★★★ 批跑 judge | Apache-2.0 | [GitHub](https://github.com/lmarena/arena-hard-auto) |

---

## 3. 重点框架（能力 · 集成切入点 · 不适用处）

### Langfuse（主拼图）

- **能力**：Datasets 版本化实验输入；Experiments（UI/SDK）对比 prompt/model/代码变更；Scores 统一承载人工/LLM/代码评判；LLM-as-judge 可跑在 trace、observation 或 experiment item 上 ([core concepts](https://langfuse.com/docs/evaluation/core-concepts))。
- **集成**：xylitol 已有 OTEL → Langfuse；eval run 对每个 dataset item 调 `cargo run`/harness，trace 自动入库；evaluator 用 [code evaluators](https://langfuse.com/docs/evaluation/evaluation-methods/code-evaluators) 或 [scores-via-sdk](https://langfuse.com/docs/evaluation/evaluation-methods/scores-via-sdk) 写回；CI 用 [Experiments in CI/CD](https://langfuse.com/docs/evaluation/experiments/experiments-ci-cd) + `langfuse/experiment-action`。
- **不适用**：不替代 SWE-bench/Terminal-Bench 的 Docker 沙箱与测试闸；需自建「任务执行器」把 xylitol 接到 dataset item。

### Promptfoo

- **能力**：YAML 声明 eval/red team；agent 场景支持 `trajectory:tool-used` 等断言；内置 OTLP receiver，可把 provider spans 关联到 test case ([tracing](https://www.promptfoo.dev/docs/tracing/))。
- **集成**：自定义 JS/Python provider 包装 xylitol CLI；OTLP child spans 可与 xylitol OTEL 并存（需统一 trace context）；CI 用 [GitHub Action](https://www.promptfoo.dev/docs/integrations/github-action/) 或 [CI/CD 指南](https://www.promptfoo.dev/docs/integrations/ci-cd/)。
- **不适用**：观测主栈会与 Langfuse 重复；更适合安全回归/trajectory 断言，而非日常 eval SSOT。

### Harbor + Terminal-Bench

- **能力**：Harbor 为 TB 2.x 官方 harness，任务=指令+容器环境+验证器；支持本地/云并行 ([Harbor docs](https://www.harborframework.com/docs/getting-started))；TB 2.1 为 89 项长程终端任务 ([tbench.ai](https://www.tbench.ai/))。
- **集成**：将 xylitol 注册为 Harbor agent adapter，跑 `harbor run -d terminal-bench/terminal-bench-2-1`；结果 reward 摘要写入 Langfuse experiment score（外挂，非原生）。
- **不适用**：重依赖 Docker/算力；对个人 harness 是「周期性对标」而非每次 commit 默认跑全量。

### SWE-bench Verified + mini-SWE-agent

- **能力**：500 项人工校验 issue；Docker harness `swebench.harness.run_evaluation` 以测试通过为闸 ([harness](https://www.swebench.com/SWE-bench/reference/harness/))；Verified 页用 mini-SWE-agent 作 LM 对标基线 ([verified](https://www.swebench.com/verified))。
- **集成**：xylitol 产出 patch predictions → 同一 harness 评分；可用 mini 脚本的批跑模式作参照实验 ([mini SWE-bench 文档](https://mini-swe-agent.com/latest/usage/swebench/))。
- **不适用**：测的是「修 repo」而非 TUI/Print 产品面；全量成本高。

### DeepEval

- **能力**：`@observe` 产生 trace tree；agent metrics（TaskCompletion、ToolCorrectness 等）；声明支持 CI/CD ([introduction](https://deepeval.com/docs/introduction))；可消费外部 OTEL spans ([tracing doc](https://deepeval.com/docs/evaluation-llm-tracing))。
- **集成**：Rust 侧继续 OTEL 导出 Langfuse；Python 薄包装用 DeepEval 读 OTEL/导出 span 做 pytest 断言，分数 `langfuse.score()` 回写。
- **不适用**：Python 中心；与 Langfuse Experiments 功能重叠，宜作「本地 pytest 层」而非第二平台。

### Braintrust

- **能力**：离线 experiment + 在线 scoring + `autoevals` 预置 scorer；明确「playground → experiment → CI → production score」闭环 ([evaluate](https://www.braintrust.dev/docs/evaluate))。
- **集成**：理论上可并行，但对 xylitol 会引入第二套 trace/dataset 存储。
- **不适用**：与「复用 Langfuse traces」约束冲突；autoevals 明确只评 span 非整条 agent trace ([autoevals](https://www.braintrust.dev/docs/evaluate/autoevals))。

### Inspect AI + Inspect Evals

- **能力**：agent（ReAct、multi-agent、Agent Bridge）、Docker 沙箱、checkpoint；Inspect Evals 打包 SWE-bench/GAIA 等 ([AISI 公告](https://www.aisi.gov.uk/blog/inspect-evals))。
- **集成**：可通过 Agent Bridge 接外部 agent；适合借 benchmark 定义，不适合当 xylitol 日常 eval UI。
- **不适用**：自带 View/VS Code 扩展；与「不要第二套 Inspect UI」冲突。

### OpenAI Evals

- **能力**：开源 registry + hosted Evals API（graders、`string_check` 等）。
- **集成**：可用 completion function 接 agent，但平台 **2026-10 只读、11 月关停** ([deprecation](https://developers.openai.com/api/docs/guides/evals))。
- **不适用**：长期 roadmap 依赖风险高；且 xylitol 非 OpenAI-only。

### Anthropic 指南（无官方 harness）

- **能力**：capability vs regression eval；代码/模型/人工三种 grader；推荐「评产出不评路径」；附录点名 Harbor、Braintrust、Langfuse ([指南](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents))。
- **集成**：直接指导 xylitol dataset 设计与 grader 选择；SWE-bench Verified / Terminal-Bench 作 coding agent 确定性 grader 范例。
- **不适用**：不提供可执行框架。

### Ragas / LiveCodeBench / Arena-Hard / WebArena / AgentBench

- **Ragas**：agent tool/goal metrics 齐全 ([agents metrics](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/agents/))，但偏 Python 离线，与 Langfuse 需胶水层。
- **LiveCodeBench**：竞赛题代码能力滚动评测 ([GitHub](https://github.com/livecodebench/livecodebench))，非 repo/TUI agent 端到端。
- **Arena-Hard**：LLM judge 比单轮回答 ([README](https://github.com/lmarena/arena-hard-auto))，非 tool/agent harness。
- **WebArena**：Web 导航 812 任务 ([GitHub](https://github.com/web-arena-x/webarena))，coding 弱相关。
- **AgentBench**：8 环境通用 agent ([GitHub](https://github.com/THUDM/AgentBench/))；OS 子集有参考价值但整体偏离个人 coding harness。

---

## 4. xylitol 分阶段拼图（M0–M3，产品语言）

| 阶段 | 目标 | 拼图 |
|------|------|------|
| **M0 — 可观测回归种子** | 把真实失败变成可重复案例 | 从 Langfuse 生产/开发 trace 抽样 → Dataset；每项附 input（用户任务）、expected（测试/补丁闸）、metadata（模型/commit）；Scores 记录「是否解决」 |
| **M1 — 开发者后置实验** | 改 prompt/工具/模型前可对比 | Langfuse Experiments SDK：task=调 xylitol harness（Print 或非交互 CLI）；evaluator=代码闸（测试通过、文件 diff 检查）+ 可选 LLM-as-judge；UI 对比 run |
| **M2 — CI 回归闸** | PR 不静默退化 | `langfuse/experiment-action` 跑回归 dataset；`RegressionError` 阈值；OTEL trace 与 experiment item 关联；可选 Promptfoo red team 夜间 job |
| **M3 — 外部对标刻度** | 与社区刻度对齐，非日常默认 | Harbor 周期性跑 SWE-bench Verified 子集 + Terminal-Bench 2.x；摘要 score 写回 Langfuse experiment；参考 Anthropic 指南区分 capability（低 pass 率探索）与 regression（近 100%）集 |

---

## 5. 风险与反模式

| 风险 / 反模式 | 说明 |
|---------------|------|
| **第二观测栈** | 同时上 Braintrust/LangSmith/Inspect View 作主 eval UI，与现有 Langfuse 分叉 ([Anthropic 附录亦建议组合但投入应在 task/grader](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)) |
| **把 leaderboard 当日常 CI** | SWE-bench Verified / Terminal-Bench 全量 Docker 成本高，宜 M3 周期性而非每 PR |
| **依赖将关停的 OpenAI Evals Platform** | [2026 年关停时间线](https://developers.openai.com/api/docs/guides/evals) |
| **评路径不评产出** | Anthropic 明确反对 rigid tool-sequence grading；应用测试/artifact 闸 |
| **LLM judge 无校准** | Langfuse LLM-as-judge 适合主观维，但需与代码闸混用 ([LLM-as-a-judge](https://langfuse.com/docs/evaluation/evaluation-methods/llm-as-a-judge)) |
| **Promptfoo 替代 Langfuse** | Promptfoo tracing 服务 eval 会话，不是长期 trace 仓库 ([tracing](https://www.promptfoo.dev/docs/tracing/)) |
| **竞赛题 benchmark 替代 agent eval** | LiveCodeBench 测代码生成，不覆盖 repo 修复/TUI 驱动 ([GitHub](https://github.com/livecodebench/livecodebench)) |
| **开箱默认 eval** | 与 xylitol「开发者/CI 后置」定位冲突；应文档化 `just eval-*` 而非 TUI 默认流程 |

---

## 参考索引（一手 URL）

- Langfuse：[Evaluation overview](https://langfuse.com/docs/evaluation/overview) · [Datasets](https://langfuse.com/docs/evaluation/experiments/datasets) · [Experiments SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) · [Experiments CI/CD](https://langfuse.com/docs/evaluation/experiments/experiments-ci-cd) · [LLM-as-a-judge](https://langfuse.com/docs/evaluation/evaluation-methods/llm-as-a-judge) · [Scores via SDK](https://langfuse.com/docs/evaluation/evaluation-methods/scores-via-sdk) · [OpenTelemetry](https://langfuse.com/integrations/native/opentelemetry)
- Promptfoo：[GitHub](https://github.com/promptfoo/promptfoo) · [Tracing](https://www.promptfoo.dev/docs/tracing/) · [Red team agents](https://www.promptfoo.dev/docs/red-team/agents/) · [CI/CD](https://www.promptfoo.dev/docs/integrations/ci-cd/)
- Harbor / Terminal-Bench：[Harbor](https://github.com/harbor-framework/harbor) · [Harbor docs](https://www.harborframework.com/docs/getting-started) · [tbench.ai](https://www.tbench.ai/) · [terminal-bench-2-1](https://github.com/harbor-framework/terminal-bench-2-1)
- SWE-bench：[GitHub](https://github.com/SWE-bench/SWE-bench/) · [Verified](https://www.swebench.com/verified) · [Harness](https://www.swebench.com/SWE-bench/reference/harness/) · [mini-SWE-agent](https://github.com/SWE-agent/mini-swe-agent)
- Anthropic：[Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- Inspect：[inspect.aisi.org.uk](https://inspect.aisi.org.uk/) · [Inspect Evals](https://www.aisi.gov.uk/blog/inspect-evals)
- DeepEval：[Introduction](https://deepeval.com/docs/introduction) · [Agent evals](https://deepeval.com/docs/getting-started-agents)
- 其他： [Braintrust evaluate](https://www.braintrust.dev/docs/evaluate) · [OpenAI evals deprecation](https://developers.openai.com/api/docs/guides/evals) · [Ragas agents](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/agents/) · [LiveCodeBench](https://github.com/livecodebench/livecodebench) · [Arena-Hard](https://github.com/lmarena/arena-hard-auto)
