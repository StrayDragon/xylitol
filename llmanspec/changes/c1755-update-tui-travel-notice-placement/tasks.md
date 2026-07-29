# Tasks: c1755-update-tui-travel-notice-placement

## 1. Specs + 绑定

- [x] 1.1 修订 live `app-tui-session-tree`：travel 后 `history @` MUST 尾随可滚；rebuild MUST NOT 顶插该 banner
- [x] 1.2 修订 live `app-tui-transcript`：重建路径对导航/瞬时通知 MUST NOT prepend（政策）
- [x] 1.3 对应 `*.feature` 场景（`@req`）+ `llman sdd validate … --strict --no-check`
- [x] 1.4 `llman sdd change start c1755-update-tui-travel-notice-placement` → branch `sdd/c1755-…`（Stage: full）

## 2. 产品实现

- [ ] 2.1 `rebuild_scrollback_from_travel` 去掉 System banner，只投影路径
- [ ] 2.2 `apply_session_tree_travel` 重建后 `push_system_note(history @ …)`
- [ ] 2.3 确认 fork / switched / debug：**无**额外 `history @`；既有尾随 note 保留
- [ ] 2.4 更新 `harness_enter_travel_closes_tree` 等断言（末尾/跟底可见；非 entries[0] 顶插）

## 3. Demo

- [ ] 3.1 `agent_demo` travel：path 后再尾随 `history @`
- [ ] 3.2 更新 `agent_demo_test` travel 断言

## 4. Gate

- [ ] 4.1 相关 `cargo test` / harness / demo 测
- [ ] 4.2 `llman sdd validate c1755-update-tui-travel-notice-placement --strict`
- [ ] 4.3（人类）TTY 手测：长史 travel 后输入框上方可见 `history @`；fork 后仅 `forked →`（无双 banner）
