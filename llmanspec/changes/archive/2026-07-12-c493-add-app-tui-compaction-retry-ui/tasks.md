# Tasks — c493-add-app-tui-compaction-retry-ui

- [x] 1.1 bridge：`CompactionEnd` / `AutoRetryEnd` 在 Busy 时恢复 `Working`
- [x] 1.2 bridge 单测：compaction Start/End（含 aborted）与 retry Start/End（成败）
- [x] 1.3 chrome：确认 Compacting/Retry 仍单行（复用或补轻量 render 断言）
- [x] 2.1 落地 `design/compaction-status.md` + `DESIGN.md` 指针（去草稿）
- [x] 3.1 `llman sdd validate c493-add-app-tui-compaction-retry-ui --strict`
- [x] 3.2 `just lint` + 相关 `cargo test`（bridge / tui）
