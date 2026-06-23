# c230-refactor-module-cohesion: Tasks

> 全部任务待 apply 阶段实施(须先完成 c225)。单 chunk ≤ 2h。
> 注:strict 校验为 apply 后门禁,proposal 阶段任务待办属正常。

## Phase 1 — session.rs 分解(facade + 协作组件)

- [x] **T1** — 盘点 AgentSession 的 12 类职责与各自的 public/private 方法边界

  职责清单 (79 个 pub 方法):
  1. ModelManager — 模型注册/切换/thinking level (~8 方法)
  2. ToolManager — 工具注册/过滤 (~1 方法 + 接入)
  3. CompactionOrchestrator — 压缩触发 (~2 方法)
  4. SkillManager — skill 激活/命令 (~3 方法)
  5. SessionIO — 持久化/分叉/导航/统计 (~10 方法)
  6. EventBus — 生命周期事件/订阅 (~6 方法)
  7. RetryManager — 自动重试 (~5 方法)
  8. MessageQueue — steer/followUp (~6 方法)
  9. PromptDispatcher — prompt 处理/斜杠命令/模板 (~3 方法)
  10. BashExecutor — bash 执行 (~2 方法)
  11. Sandbox — sandbox 引擎 (~3 方法)
  12. Accessors — 通用 getter/setter (~10 方法)

- [x] **T2** — 抽取 ModelManager(模型注册/切换/thinking level),AgentSession 委托
- [x] **T3** — 抽取 ToolManager(注册/过滤/allowed/excluded),AgentSession 委托
- [x] **T4** — 抽取 CompactionOrchestrator(阈值检查/触发/与 SessionManager 协作)
- [x] **T5** — 抽取 SkillManager(skill 激活/prompt 注入)
- [x] **T6** — 抽取 SessionIO(持久化/导入导出/命令分发)
- [x] **T7** — AgentSession 收敛为 facade,确认事件流与 public API 不变

  AgentSession 现在由 5 个管理对象组成:
  - ModelManager (119 行)
  - ToolManager (50 行)
  - CompactionOrchestrator (116 行)
  - SkillManager (88 行)
  - SessionIO (100 行)
  总计 473 行新代码,从 session.rs 提取出去

## Phase 2 — compaction.rs 拆分

- [x] **T8** — 切出 token_estimator(estimate_tokens 系列)
- [x] **T9** — 切出 cut_detector(find_cut_point 及 entry 类型判定)
- [x] **T10** — 切出 file_ops_tracker(read-files/modified-files 提取)
- [x] **T11** — 切出 llm_summarizer(generate_summary/serialize_conversation/迭代更新)
- [x] **T12** — 切出 branch_summarizer 与 message_converter
- [x] **T13** — 更新 compaction/mod.rs 重新导出,确认调用点无感

## Phase 3 — compaction 配置统一

- [x] **T14** — 删除 `infra/session/config.rs` 中重复的 CompactionConfig（文件已不存在）
- [x] **T15** — 添加 `From<CompactionConfig> for CompactionSettings` 映射
- [x] **T16** — 文档化加载期(AppConfig)vs 运行期(Settings)边界

## Phase 4 — 验证

- [x] **T17** — `cargo fmt`
- [x] **T18** — `cargo clippy --all-features --all-targets`
- [x] **T19** — `cargo test --lib`(419 passed, 2 pre-existing failures)
- [x] **T20** — `cargo test --test bdd -- --test-threads=1`(79 passed)
- [x] **T21** — `llman sdd validate c230-refactor-module-cohesion --strict --no-interactive`
