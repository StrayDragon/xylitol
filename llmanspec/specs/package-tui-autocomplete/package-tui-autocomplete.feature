# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-autocomplete

  @req:ac01
  场景: async-cancellation
    假如 CombinedAutocompleteProvider 设 fd_path 且 fd 子进程查询进行中
    当 其 CancellationToken 被取消
    那么 子进程收到 SIGKILL 且 get_suggestions_async 立即返回空结果

  @req:ac01
  场景: async-fuzzy-with-fd
    假如 CombinedAutocompleteProvider 的 fd_path 指向有效 fd 二进制
    当 以 @ prefix query 调用 get_suggestions_async
    那么 fd 子进程运行且结果与 shell fd 命令同 query 一致

  @req:ac02
  场景: fd-recursive-search-finds-nested-files
    假如 tempdir 含嵌套目录与文件
    当 以 base_dir 指向 tempdir 且 query 匹配深层文件调用 walk_directory_with_fd
    那么 结果含嵌套文件路径且目录带 trailing slash

  @req:ac02
  场景: fd-path-none-fallback
    假如 fdPath 为 None
    当 调用 get_fuzzy_file_suggestions
    那么 函数回退到现有非递归 read_dir 并返回结果

  @req:ac03
  场景: debounce-drops-intermediate-calls
    假如 DebouncedAutocomplete 配置 250ms delay
    当 0ms/50ms/100ms 三次 get_suggestions 到达且仅最后一次继续
    那么 前两个 CancellationToken 被取消且仅 100ms 调用的结果在 350ms 点返回

  @req:ac03
  场景: debounce-cancellation-stops-in-flight
    假如 debounced query 已启动 fd 子进程
    当 新调用取消前一 token
    那么 旧子进程收到 SIGKILL 且新 query 运行至完成

  @req:ac03
  场景: debounce-in-test
    假如 #[tokio::test(start_paused)] 激活且 DebouncedAutocomplete delay 为 250ms
    当 0ms 到达 get_suggestions 调用
    那么 tokio::time::advance(250ms).await 触发 query 而无需真实 wall-clock 等待
