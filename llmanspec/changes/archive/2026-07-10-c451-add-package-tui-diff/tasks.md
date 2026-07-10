# Tasks — c451-add-package-tui-diff

- [x] 1. 升格 proposal + design + delta `package-tui-diff`
- [x] 2. `Cargo.toml` 加 `similar`；`components/diff.rs` + mod/lib re-export
- [x] 3. 解析 DisplayText / UnifiedText / LinePair；行级着色
- [x] 4. Word-level（相邻 -/+ 对）+ CJK `visible_width` 折行
- [x] 5. 可选 side-by-side（宽 ≥ 阈值）
- [x] 6. 单测 / snapshot：空变更、CJK、intra-line、宽度
- [x] 7. `agent_demo`：可展开 Diff 块 + 样例
- [x] 8. `PI_DELTAS.md` 记 D17；`cargo test -p xylitol-tui`；validate
