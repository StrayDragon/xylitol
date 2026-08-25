# language: zh-CN
# capability: package-tui-autocomplete
# purpose: 自动补全：CancellationToken 取消、可选 fd 模糊搜、DebouncedAutocomplete。
# scope: xylitol-tui 包, workspace 测试

功能: package-tui-autocomplete

  @req:ac01 @human
  场景: async-cancellable-autocomplete
    - autocomplete 系统 MUST 支持异步建议获取，基于 CancellationToken 取消，使用户输入新内容时可中止进行中的文件系统查询，镜像 pi 的 AbortSignal 模式。CombinedAutocompleteProvider MUST 在其 fuzzy file search 路径暴露接受 CancellationToken 的 async 方法。

  @req:ac02 @human
  场景: walk-directory-with-fd
    - autocomplete 系统 MUST 支持递归模糊文件系统搜索，spawn fd(1) 子进程，参数匹配 pi 的 walkDirectoryWithFd：--base-directory、--max-results、--type f、--type d、--follow、--hidden，query 含 path separator 时使用 --full-path。fdPath MUST 可选（None = 无 fd，回退 read_dir stub）。CancellationToken 取消时子进程 MUST 收到 SIGKILL。

  @req:ac03 @human
  场景: debounced-autocomplete
    - DebouncedAutocomplete<P> wrapper MUST 按配置 delay（默认 250ms，对齐 pi setTimeout）debounce 连续 get_suggestions 调用。窗口内新调用到达时 MUST 取消前一 CancellationToken，仅最新 query 继续。debounce MUST 经 #[tokio::test(start_paused)] 可确定性测试窗口行为。
