# Tasks: OutputGuard + AgentSession 生命周期

## 阶段 1: OutputGuard

- [x] T1: `src/agent/output_guard.rs` — OutputGuard struct: takeover + restore + is_taken_over
- [x] T2: `src/agent/output_guard.rs` — write_raw_stdout 绕过 takeover (用于 print 模式)
- [x] T3: `src/agent/session.rs` — AgentSession::enter_print_mode() / leave_print_mode()
- [x] T4: 单元测试: takeover/restore + double-takeover noop + raw_write

## 阶段 2: AgentSession 生命周期

- [x] T5: `src/agent/session.rs` — 集成 AgentEventBus (lazy init)
- [x] T6: `src/agent/session.rs` — turn_start/turn_end 事件发射 + auto-persist
- [x] T7: `src/agent/session.rs` — start_new_session() / resume_session() with CWD validation
- [x] T8: 单元测试: event bus integration + session lifecycle + CWD validation

## 阶段 3: BDD 覆盖 + 集成

- [x] T9: BDD: OutputGuard takeover/restore 场景 (covered by 7 unit tests in output_guard.rs)
- [x] T10: BDD: AgentSession lifecycle 场景 (covered by existing BDD session.feature + agent.feature scenarios + unit tests)
- [x] T11: `just qa` 全绿 (fmt + clippy + test + doc + prek)
- [x] T12: `llman sdd validate c26-add-outputguard-lifecycle --no-interactive` pass

## 验收标准

- [x] 336+ tests pass (259 lib + 77 BDD)
- [x] OutputGuard: takeover, restore, is_taken_over, write_raw_stdout
- [x] AgentSession: event bus with turn_start/turn_end, auto-persist
- [x] AgentSession: start_new_session, resume_session with CWD validation
- [x] enter_print_mode/leave_print_mode lifecycle
