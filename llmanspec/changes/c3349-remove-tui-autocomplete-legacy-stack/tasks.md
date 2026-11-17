# Tasks

- [ ] T1 搬迁词汇表与帮助函数
  - AutocompleteItem / SlashCommand / AutocompleteSuggestions 与 extract_at_prefix / extract_dollar_prefix / parse_path_prefix / expand_home_path / path_autocomplete_item 从 src/autocomplete.rs 迁入 src/completion.rs
  - src/lib.rs re-export 来源路径更新，公开导出名单不变
  - editor/autocomplete.rs 与包内测试的 import 更新（纯路径机械改）
  - 验证：`just test-tui` 全绿（change 分支上相对 merge-base 行为零变化）

- [ ] T2 删除旧栈三件与测试拆解 [blocked-by: T1]
  - 删 AutocompleteProvider / CombinedAutocompleteProvider（含旧 fd 评分副本）/ DebouncedAutocomplete
  - tests/autocomplete_test.rs 保留 build_fd_path_query / fd 递归搜索 / 取消 SIGKILL 用例，删除旧栈构造用例
  - 验证：change 分支上 `rg 'AutocompleteProvider|CombinedAutocompleteProvider|DebouncedAutocomplete' crates src tests` 于 feature 文件外零命中；`just test-tui` 绿；包实例 `llman-sdd validate --check`（在 crates/xylitol-tui 跑）绿；根 `llman-sdd validate c3349-remove-tui-autocomplete-legacy-stack --strict --no-check` 绿
