# Tasks — TUI @ 路径补全接入 fd

- [x] 1. 引擎：`fd_path_suggestions`（parse_path_prefix → fd walk → fuzzy 评分 → path item）。
- [x] 2. 引擎：AtPathSource `fd_path` + `new_with_fd`；suggestions 先 fd，空则 read_dir 回落。
- [x] 3. 产品：`find_fd()` 查 `fd` / `fdfind`；install_completion_sources 接线。
- [x] 4. 单测：fd 深层召回、无 fd 回落空。
- [x] 5. Specs：app-tui-input at-path-fd-completion。
