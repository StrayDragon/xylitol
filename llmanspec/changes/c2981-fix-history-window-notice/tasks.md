# Tasks

- [ ] 1. **specs**（spec）：`app-tui-fixed-zone` 的 `r2936` 区分「加载更早」与「更早内容已收起」两种提示，剩余量用定性文案；新增「窗口溢出后提示为收起而非加载更早」场景。测：`llman-sdd validate --all --strict`。
- [ ] 2. **history_window**（tui）：新增 `collapsed_notice_text` / `show_notice`；`trim_to_window` 在仅溢出时使用收起文案；`LoadedHistory::merge_older` 按 entry id 去重。测：`cargo test -p xylitol-app-tui history_window`。
- [ ] 3. **session_ops**（tui）：在切换/恢复/旅行/fork/debug 场景初始化 `loaded_history`；`prepend_history` 用去重合并并保留提示；新增 `expand_history_window` 从已加载缓冲恢复被裁页。测：`cargo test -p xylitol-app-tui`。
- [ ] 4. **history effects**（tui）：盘上仍有更早条目时回源加载；否则从缓冲展开；点击均保持锚点。测：`cargo test -p xylitol-app-tui`。
- [ ] 5. **bdd**（repo）：补窗口溢出收起提示场景绑定与步骤。测：`cargo test --all-features --test bdd`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
