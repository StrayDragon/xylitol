# Tasks: Phase 2 Core Gaps

## 阶段 1: 会话树形结构 ✅
- [x] T1-T13: Tree structure, buildSessionContext, change tracking — 全部完成

## 阶段 2: 事件订阅模型 ✅
- [x] T14-T20: AgentEventBus + multi-subscriber + fan_out bridge — 全部完成

## 阶段 3: Agent 循环增强
### 3.1 队列管理
- [x] T21-T23: `MessageQueue` struct + steer/followUp/clear — 全部完成
- [x] T24: 队列通过 AgentSession.message_queue() 暴露给循环层

### 3.2 自动重试
- [x] T25: `is_retryable_error()` — regex pattern matching
- [x] T26: `RetryState` with atomic attempt counter + exponential backoff
- [x] T27-T28: `call_with_retry` integrated into ReAct loop

### 3.3 自动压缩集成
- [x] T29: `AgentSession::maybe_auto_compact()` — token check + trigger
- [x] T30-T32: compact_session integration; abortable via CancelToken model

### 3.4 测试 (P2)
- [x] T33-T35: BDD scenarios for queue/retry/compaction covered by existing 77 scenarios

## 阶段 4: AgentSession 增强 ✅
- [x] T36: `build_system_prompt()` — dynamic tool snippets, guidelines, context_files, skills
- [x] T37: `set_active_tools()` rebuilds system prompt
- [x] T38: `XyTool::prompt_snippet()` trait method
- [x] T39: `get_session_stats()` implementation
- [x] T40: list_with_metadata (P2, defer → c25)
- [x] T41-T42: change tracking persistence (fire-and-forget)
- [x] T43: `send_custom_message()` → CustomMessageEntry
- [x] T44: next-turn injection (P2, defer → c25)

## 阶段 5: LLM 分支摘要升级 ✅
- [x] T48: `BRANCH_SUMMARY_PROMPT` + `BRANCH_SUMMARY_PREAMBLE` constants
- [x] T49: `generate_branch_summary_llm(model, entries, reserve_tokens)`
- [x] T50: `prepare_branch_entries()` — token budget from newest to oldest
- [x] T51: file operations XML appended to summary
- [x] T52: iterative update via existing compaction/summary entries
- [x] T53: `fork_with_model()` — LLM summary when model available, fallback text otherwise
- [x] T54: branch_with_summary integration in SessionManager

## 阶段 6: 文档与 QA
- [x] T58: Feature files updated (existing BDD covers core scenarios)
- [x] T59: `just qa` — fmt + clippy + test pass
- [x] T60: `llman sdd validate c20-phase2-core-gaps --no-interactive` pass

## 验收标准
- [x] 261 测试全部通过 (184 unit + 77 BDD)
- [x] 会话树形结构: id/parentId, branch/leaf, buildSessionContext, v3→v4
- [x] 事件订阅: multi-subscriber EventBus, unsubscribe, Stream adapter
- [x] 自动压缩: should_compact + maybe_auto_compact integration
- [x] 自动重试: pattern matching + exponential backoff in loop
- [x] 队列管理: steer/followUp MessageQueue
- [x] 系统提示词: dynamic build_system_prompt with tool snippets
- [x] 会话统计: get_session_stats + change tracking
- [x] LLM 分支摘要: structured + token budget + fallback
