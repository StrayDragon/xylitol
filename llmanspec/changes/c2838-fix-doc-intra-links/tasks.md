# Tasks — c2838 doc intra-doc 链接治理

- [x] 完成调研（28 处定位、语义核对、修复方案表，产出 research/findings.md）
- [ ] 修复 BROKEN ×18（agent/model/bootstrap/driver/tui/process 各区 doc 链接）
- [ ] 修复 PRIVATE ×10（改文本）
- [ ] 移除 src/lib.rs 两个 `#![allow(rustdoc::…)]`
- [ ] `cargo doc --all-features --no-deps` 零 warning
- [ ] `just qa quiet` 全绿 + `llman-sdd validate --strict` 绿
