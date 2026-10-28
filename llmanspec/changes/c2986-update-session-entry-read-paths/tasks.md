# Tasks

- [ ] 1. **audit**（tui）：逐路径核对 `session_entries` 调用点，结论写入 `design.md`（tree 打开已走 `SessionTree`；travel/fork/compact/estimate 保留全量并记录理由）。测：`llman-sdd validate c2986-update-session-entry-read-paths`。
- [ ] 2. **footer**（tui）：`footer_entries_available` 改为 `session_entries_page(Some(1))`，只判断是否存在条目。测：`cargo test -p xylitol-app-tui footer`。
- [ ] 3. **cli restore**（tui）：无本地 store 的 CLI 恢复回退改为按 `HISTORY_PAGE` 取一页并透传 `has_more`。测：`cargo test -p xylitol-app-tui`。
- [ ] 4. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
