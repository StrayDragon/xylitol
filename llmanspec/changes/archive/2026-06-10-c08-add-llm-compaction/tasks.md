# Tasks: LLM 上下文压缩

## P0: 切点检测

- [x] T1: 实现 `find_cut_point(entries, start, end, keep_tokens) -> CutPointResult` — `src/infra/session/compaction.rs`
- [x] T2: 实现 `estimate_tokens_entry(entry: &SessionEntry) -> u64` 估算
- [x] T3: 单元测试: empty、all-user、user+assistant+tool-result 混合

## P0: 结构化摘要生成

- [x] T4: `generate_summary(messages, model, reserve_tokens, prev_summary?)` — 系统提示 + 对话序列化 + LLM 调用
- [x] T5: `serialize_conversation(messages: &[XyContent]) -> String`
- [x] T6: 添加 SUMMARIZATION_PROMPT / UPDATE_SUMMARIZATION_PROMPT 常量
- [x] T7: 单元测试: basic、iterative update、empty messages

## P0: 文件追踪

- [x] T8: `extract_file_ops(messages, prev_compaction?) -> FileOps`
- [x] T9: 将 `<read-files>` / `<modified-files>` XML tags 附加到 summary 末尾
- [x] T10: 单元测试: read+write+edit 混合、与 previous 合并

## P0: 主流程

- [x] T11: `compact_session(mgr, session_id, model, settings) -> Result<CompactionEntry>`
- [x] T12: 定义 `CompactionSettings` 并映射 AppConfig

## P1: Agent 集成

- [x] T13: `AgentSession::compact_current_session(&self, model) -> Result<()>`
- [x] T14: `AgentLoop::run()` 中 turn 结束后触发 shouldCompact（defer → c15-add-session-fork）
- [x] T15: AgentEvent 发出 CompactionStart / CompactionEnd (已存在枚举变体)

## P1: BDD

- [x] T16: 实现 `tests/features/compaction.feature` 5 个场景 (已有步骤定义, 5/5 通过)
- [x] T17: `cargo test --test bdd compaction -- --test-threads=1` 通过

## 验证

- [x] T18: `cargo test -p xylitol` 通过 (250/250)
- [x] T19: `just qa` 通过
- [x] T20: `llman sdd validate c08-add-llm-compaction --strict --no-interactive`
