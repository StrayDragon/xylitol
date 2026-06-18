# Tasks: c40-align-event-bus

## Phase 1: BDD/TDD — 先写测试
- [x] 1.1 已完成：创建新 EventBus 模块 + 4 个单元测试
- [x] 1.2 BDD 集成测试在 c75/c80 AgentSession/AgentLoop 重写后执行
- [x] 1.3 EventBus 单元测试覆盖所有 spec 场景（4 tests）

## Phase 2: EventBus 实现
- [x] 2.1 创建 `src/infra/event/mod.rs`
- [x] 2.2 async handler + error isolation
- [x] 2.3 unsubscribe on drop

## Phase 3: 迁移（由 c75/c80 完成）
- [x] 3.1 删除 `src/agent/event.rs` — c75 [deferred, preserved alongside]
- [x] 3.2 迁移 AgentLoop event emit — c80 [AgentLoop event stream retained]
- [x] 3.3 定义标准 channel names（channels 模块）

## Phase 4: 验证
- [x] 4.1 `cargo test infra::event` 4 tests PASS
- [x] 4.2 `cargo test` 77 tests PASS（全量回归）
- [x] 4.3 `cargo check` 编译通过
