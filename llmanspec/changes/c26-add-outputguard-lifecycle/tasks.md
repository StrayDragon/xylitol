# Tasks: OutputGuard + AgentSession 生命周期

## 阶段 1: OutputGuard

- [ ] T1: `src/agent/output_guard.rs` — OutputGuard struct: takeover + restore + is_taken_over
- [ ] T2: `src/agent/output_guard.rs` — write_raw_stdout 绕过 takeover (用于 print 模式)
- [ ] T3: `src/agent/session.rs` — AgentSession::enter_print_mode() / leave_print_mode()
- [ ] T4: 单元测试: takeover/restore + double-takeover noop + raw_write

## 阶段 2: AgentSession 生命周期

- [ ] T5: `src/agent/session.rs` — 集成 AgentEventBus (lazy init)
- [ ] T6: `src/agent/session.rs` — turn_start/turn_end 事件发射 + auto-persist
- [ ] T7: `src/agent/session.rs` — start_new_session() / resume_session() with CWD validation
- [ ] T8: 单元测试: event bus integration + session lifecycle + CWD validation

## 阶段 3: BDD 覆盖 + 集成

- [ ] T9: BDD: OutputGuard takeover/restore 场景
- [ ] T10: BDD: AgentSession lifecycle 场景
- [ ] T11: `just qa` 全绿 (fmt + clippy + test + doc + prek)
- [ ] T12: `llman sdd validate c26-add-outputguard-lifecycle --no-interactive` pass

## 验收标准

- [ ] 340+ tests pass
- [ ] OutputGuard: takeover, restore, is_taken_over, write_raw_stdout
- [ ] AgentSession: event bus with turn_start/turn_end, auto-persist
- [ ] AgentSession: start_new_session, resume_session with CWD validation
- [ ] enter_print_mode/leave_print_mode lifecycle
