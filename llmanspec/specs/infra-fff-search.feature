# language: zh-CN
# purpose: fff 工具源：默认装配的 ffgrep / fffind，按工作区惰性建索引，插件缺失回落内置检索。

功能: infra-fff-search

  @req:r2979
  规则: fff-tool-surface
    tools-fff 插件 须提供 ffgrep 与 fffind 两个 XyTool。ffgrep 须接受 query（必填）、path、glob、mode（smart / literal / regex / fuzzy）、definitionsOnly、context、limit、timeout；输出 须含相对路径、行号与匹配行。fffind 须接受 query（必填）、path、mode（fuzzy / glob / regex）、limit、timeout；输出 须为相对路径列表。limit 须钳制：ffgrep 1..200，fffind 1..1000；timeout 须遵循全局工具超时语义（正整数秒、钳制 600s、默认 60s）。ffgrep 须声明 replaces grep，fffind 须声明 replaces find。

    场景: ffgrep 内容检索
      假如 会话工作区存在文件 "src/main.rs" 内容为 "fn main() {}"
      当 以 ffgrep 检索 "fn main"
      那么 输出含 "src/main.rs:" 与 "fn main"

    场景: fffind 路径检索
      假如 会话工作区存在文件 "src/lib.rs"
      当 以 fffind 检索 "lib"
      那么 输出含 "src/lib.rs"

    场景: ffgrep 拒绝零超时
      假如 会话工作区存在文件 "src/main.rs"
      当 以 ffgrep 检索 "fn main" 且 timeout 为 0
      那么 ffgrep 以无效 timeout 错误拒绝

  @req:r2980
  规则: fff-index-lifecycle
    索引 须按会话工作区惰性建立并在进程内复用；不得在仓库内写入索引文件。首次扫描 须有上限（默认 10s）；超时 须继续在已索引部分搜索并标注 partial，不得无限阻塞。搜索 须在阻塞线程池执行，不得阻塞 async runtime。不同工作区 须各自持有独立索引，不得串用结果。

    场景: 索引按工作区隔离
      假如 两个会话工作区分别存在不同文件
      当 分别以 ffgrep 检索各自文件
      那么 各自输出只含本工作区路径

  @req:r2981
  规则: fff-fallback
    tools-fff 未装配或 feature fff 关闭时，内置 grep / find 须保持可用；不得因 fff 缺失使工具集为空或启动失败。fff 索引初始化失败时 工具 须返回可读错误，不得静默改用内置语义。

    场景: 插件缺失回落内置
      假如 tools-fff 插件未装配
      当 装配工具集
      那么 含 grep 与 find 且不含 ffgrep
