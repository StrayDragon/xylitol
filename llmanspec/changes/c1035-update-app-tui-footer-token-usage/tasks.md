# Tasks — c1035-update-app-tui-footer-token-usage

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1035-update-app-tui-footer-token-usage --no-interactive`（`--strict` 会因未完成 apply 任务报 pending ERROR，属预期）
- [x] 2. `footer_token_label(provenance, tokens)` 纯函数 + 单测（Api→`used N`；Heuristic→`~`；Unknown→`?`）
- [x] 3. UiModel / footer 状态携带可选 token 字段；`format_footer_text` 接入字段序
- [x] 4. host/effects：turn 结束、travel、compact 后调用 `Driver::estimate_context_tokens` 刷新
- [x] 5. harness：空会话省略；Fake 消息后出现 used；Heuristic 路径含 `~`；travel 换叶更新
- [x] 6. 同步 `src/app/tui/design/footer.md`；`just lint` / 相关 test
