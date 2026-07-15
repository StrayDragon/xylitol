# ACCEPTANCE — c1020-add-app-tui-session-lifecycle-slash

## 自动

```bash
cargo test --lib -- --test-threads=1 h33_slash h34_slash h35_slash
cargo test --lib arch_guard -- --test-threads=1
llman sdd validate c1020-add-app-tui-session-lifecycle-slash --strict --no-interactive
just lint
```

## 人类手测（归档 / commit 前）

前置：`cargo run -- --trust`

- [ ] `/` 补全含 `session-new` / `session-clone` / `session-name`
- [ ] `/session-new` → 空会话；`/new` → unknown
- [ ] 有历史时 `/session-clone` → 新会话保留到 leaf 的路径；`/clone` → unknown
- [ ] 空会话 `/session-clone` → Nothing to clone yet
- [ ] `/session-name` → usage；`/session-name foo` → set；再 `/session-name` → 显示
- [ ] `/name` → unknown
- [ ] `/session-fork` 仍按 A02（user→Before）；与 clone 语义不混

## 下一步

1. 人类勾选手测（或授权跳过）
2. `llman-sdd-archive` → 合并主 specs
3. commit（建议消息见下）

```
feat(tui): session-new/clone/name lifecycle slash (c1020)
```
