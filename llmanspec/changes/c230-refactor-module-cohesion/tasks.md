# c230-refactor-module-cohesion: Tasks

> 全部任务待 apply 阶段实施(须先完成 c225)。单 chunk ≤ 2h。
> 注:strict 校验为 apply 后门禁,proposal 阶段任务待办属正常。

## Phase 1 — session.rs 分解(facade + 协作组件)

- [ ] **T1** — 盘点 AgentSession 的 12 类职责与各自的 public/private 方法边界
- [ ] **T2** — 抽取 ModelManager(模型注册/切换/thinking level),AgentSession 委托
- [ ] **T3** — 抽取 ToolManager(注册/过滤/allowed/excluded),AgentSession 委托
- [ ] **T4** — 抽取 CompactionOrchestrator(阈值检查/触发/与 SessionManager 协作)
- [ ] **T5** — 抽取 SkillManager(skill 激活/prompt 注入)
- [ ] **T6** — 抽取 SessionIO(持久化/导入导出/命令分发)
- [ ] **T7** — AgentSession 收敛为 facade,确认事件流与 public API 不变

## Phase 2 — compaction.rs 拆分

- [ ] **T8** — 切出 token_estimator(estimate_tokens 系列)
- [ ] **T9** — 切出 cut_detector(find_cut_point 及 entry 类型判定)
- [ ] **T10** — 切出 file_ops_tracker(read-files/modified-files 提取)
- [ ] **T11** — 切出 llm_summarizer(generate_summary/serialize_conversation/迭代更新)
- [ ] **T12** — 切出 branch_summarizer 与 message_converter
- [ ] **T13** — 更新 compaction/mod.rs 重新导出,确认调用点无感

## Phase 3 — compaction 配置统一

- [ ] **T14** — 删除 `infra/session/config.rs` 中重复的 CompactionConfig
- [ ] **T15** — 在 SessionManager 初始化处建立唯一映射 `AppConfig.compaction → CompactionSettings` 并加注释
- [ ] **T16** — 文档化加载期(AppConfig)vs 运行期(Settings)边界(代码注释或 docs)

## Phase 4 — 验证

- [ ] **T17** — `cargo fmt`
- [ ] **T18** — `cargo clippy --all-features --all-targets`
- [ ] **T19** — `cargo test --lib`(无回归)
- [ ] **T20** — `cargo test --test bdd -- --test-threads=1`(无回归)
- [ ] **T21** — `llman sdd validate c230-refactor-module-cohesion --strict --no-interactive`
