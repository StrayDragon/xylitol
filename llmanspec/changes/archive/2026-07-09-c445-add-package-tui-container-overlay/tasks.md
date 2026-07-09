# c445 Tasks — Container + OverlayHandle + image trim

> 每块 ≤2h。校验命令在末尾。

## 1. Container

- [x] 1.1 在 `tui.rs`（或 `components/container.rs`）实现 `Container`：`add_child` / `remove_child` / `clear` / `render` 垂直拼接 / `invalidate` / `handle_input` no-op
- [x] 1.2 `lib.rs` 导出 `Container`
- [x] 1.3 harness：嵌套 Text 渲染顺序断言

## 2. OverlayHandle

- [x] 2.1 为 overlay 条目分配稳定 `overlay_id`；`show_overlay` 返回 `OverlayHandle`
- [x] 2.2 实现 `hide` / `set_hidden` / `is_hidden` / `is_focused`；hide 后恢复焦点
- [x] 2.3 实现最小 `focus` / `unfocus`（无完整 focus-restore 状态机）
- [x] 2.4 harness：show → 可见；hide → 合成中消失

## 3. 裁剪（按需）

- [x] 3.1 确认 `agent_demo` / 默认测试不依赖 `Image`
- [x] 3.2 删除或 feature-gate `components/image` 与多余 `terminal_image` encode；保留 `is_image_line`
- [x] 3.3 更新 `lib.rs` 导出

## 4. 校验

- [x] 4.1 `cargo test -p xylitol-tui`
- [x] 4.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings`
- [x] 4.3 `llman sdd validate c445-add-package-tui-container-overlay --strict --no-interactive`
- [x] 4.4 更新 `_HANDOFF.md` 下一刀指针

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c445-add-package-tui-container-overlay --strict --no-interactive
```
