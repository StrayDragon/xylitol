# Tasks — c1215-add-test-bdd-solidify-pilot

试点实现已完成，tasks 为确认清单。

- [x] 新增 `llmanspec/specs/agent-runtime/agent-runtime.feature`（solidify 风格，2 场景）
- [x] `tests/bdd.rs` 新增 6 个 step（匹配 spec.toon given/when/then 文本）+ 2 个 `#[scenario]` 绑定
  （path=`llmanspec/specs/agent-runtime/agent-runtime.feature`，name=英文 id）
- [x] `cargo test --test bdd -- --test-threads=1` → 99 passed, 0 failed
- [x] `llman sdd validate agent-runtime --check` → "1 feature(s) parsed, BDD check passed"
- [x] `llman sdd validate c1215-add-test-bdd-solidify-pilot --strict` 通过
- [x] `llman sdd solidify c1215-add-test-bdd-solidify-pilot --dry-run` 确认 delta scenarios 可生成
