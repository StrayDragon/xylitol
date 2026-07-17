# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-logging

  @req:dl1
  场景: file-only-never-stdout-stderr
    假如 debug 构建或已显式打开观测且 TUI 活跃
    当 任意层发出 log::debug 或 fastrace Event
    那么 内容写入日志目录文件且 stdout/stderr 无污染

  @req:dl1
  场景: debug-default-on
    假如 cfg(debug_assertions) 且未强制关闭
    当 启动应用
    那么 file-only Reporter/logger 已装配

  @req:dl1
  场景: release-needs-opt-in
    假如 release 且未设 XYLITOL_DEBUG / XYLITOL_PROVIDER_TRACE / 等价 RUST_LOG
    当 启动应用
    那么 不安装会落盘的观测后端（或等价零写入）
