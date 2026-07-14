# ACCEPTANCE — c1065-add-app-tui-session-resume-panel

## 自动

```bash
cargo test --lib -- --test-threads=1 h32_ h36_ h37_ session_resume arch_guard
llman sdd validate c1065-add-app-tui-session-resume-panel --strict --no-interactive
just lint
```

## 人类手测（归档前建议）

前置：`cargo run -- --trust`

- [ ] `/session-resume` 开板：见 Current/All、Name、Sort、搜索提示
- [ ] Tab 切换 scope；Ctrl+S 循环 Sort；Ctrl+N Named 过滤
- [ ] 搜索 `re:` 与 `"phrase"` 收窄列表
- [ ] Enter 切换会话；Esc 不切换
- [ ] Ctrl+R rename；Ctrl+D 删除确认（当前会话应拒绝）
- [ ] Ctrl+P 切换 path；Threaded 下 fold/unfold 子会话
- [ ] `/resume` → unknown

## 下一步

1. 人类勾选手测（或授权跳过）
2. `llman-sdd-archive`（合并主 specs；可能需 `--skip-specs` 若 staleness）
3. commit（建议）:

```
feat(tui): pi-aligned session-resume panel (c1065 P0–P2)
```
