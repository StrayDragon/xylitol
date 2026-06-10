# Tasks: LLM 上下文压缩

## P0: 切点检测

- [ ] T1: 实现 `find_cut_point(entries, start, end, keep_tokens) -> CutPointResult` — `src/infra/session/compaction.rs`
- [ ] T2: 实现 `estimate_tokens_entry(entry: &SessionEntry) -> u64` 估算
- [ ] T3: 单元测试: empty、all-user、user+assistant+tool-result 混合

## P0: 结构化摘要生成

- [ ] T4: `generate_summary(messages, model, reserve_tokens, prev_summary?)` — 系统提示 + 对话序列化 + LLM 调用
- [ ] T5: `serialize_conversation(messages: &[XyContent]) -> String`
- [ ] T6: 添加 SUMMARIZATION_PROMPT / UPDATE_SUMMARIZATION_PROMPT 常量
- [ ] T7: 单元测试: basic、iterative update、empty messages

## P0: 文件追踪

- [ ] T8: `extract_file_ops(messages, prev_compaction?) -> FileOps`
- [ ] T9: 将 `<read-files>` / `<modified-files>` XML tags 附加到 summary 末尾
- [ ] T10: 单元测试: read+write+edit 混合、与 previous 合并

## P0: 主流程

- [ ] T11: `compact_session(mgr, session_id, model, settings) -> Result<CompactionEntry>`
- [ ] T12: 定义 `CompactionSettings` 并映射 AppConfig

## P1: Agent 集成

- [ ] T13: `AgentSession::compact_current_session(&self, model) -> Result<()>`
- [ ] T14: `AgentLoop::run()` 中 turn 结束后触发 shouldCompact（defer → c15-add-session-fork）
- [ ] T15: AgentEvent 发出 CompactionStart / CompactionEnd

## P1: BDD

- [ ] T16: 实现 `tests/features/compaction.feature` 5 个场景
- [ ] T17: `cargo test --test bdd compaction -- --test-threads=1` 通过

## 验证

- [ ] T18: `cargo test -p xylitol` 通过
- [ ] T19: `just qa` 通过
- [ ] T20: `cd . && llman sdd validate c08-add-llm-compaction --strict --no-interactive`
