# Tasks — c1230-remove-event-bus-orphan-feature

- [x] 确认 event-bus.feature 零引用（tests/src/llmanspec）
- [x] 删除 tests/features/event-bus.feature
- [x] delta test-bdd modify r1（更新全量通过合约，移除过时数字 77）
- [x] cargo test --test bdd → 100 passed, 0 回归
- [x] llman sdd validate c1230 --strict 通过
