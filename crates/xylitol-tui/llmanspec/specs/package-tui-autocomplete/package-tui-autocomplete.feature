# language: zh-CN
# purpose: 补全的 fd 递归模糊检索与取消：fd(1) 子进程参数契约、CancellationToken 取消与回退行为。

功能: package-tui-autocomplete

  @req:r1579
  规则: walk-directory-with-fd
    补全的递归模糊文件检索必须 spawn fd(1) 子进程，参数匹配 pi 的 walkDirectoryWithFd：--base-directory、--max-results、--type f、--type d、--follow、--hidden，query 含 path separator 时使用 --full-path。fdPath 必须可选（None = 无 fd，回退单层目录扫描）。检索必须接受 CancellationToken；取消时子进程必须收到 SIGKILL。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
