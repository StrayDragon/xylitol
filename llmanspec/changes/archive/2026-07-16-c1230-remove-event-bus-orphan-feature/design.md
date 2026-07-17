# Design — c1230-remove-event-bus-orphan-feature

## 决策

删除而非迁移。EventBus（`src/infra/event/mod.rs`）已有 7 个单测覆盖 emit/on/
unsubscribe/drop/clear——与 event-bus.feature 的 4 个场景语义完全重叠。迁移到 solidify
链路（新建 infra-event spec + step）会与单测重复，违反测试分层原则。

## 验证证据

- `rg 'event-bus' tests/ src/ llmanspec/` → 零引用（孤儿确认）
- 删除后 `cargo test --test bdd` → 100 passed, 0 回归
- EventBus 单测 `cargo test --lib infra::event` → 7 passed
