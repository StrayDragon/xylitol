# c20-add-tools Tasks

- [x] 定义 Tool trait（基于 adk-core FunctionTool）
- [x] 实现 ToolRegistry（注册、查找、列举）
- [x] 实现 read 工具（tokio::fs 读取文件）
- [x] 实现 bash 工具（tokio::process 执行命令，含超时和输出限制）
- [x] 实现 edit 工具（搜索替换模式）
- [x] 实现 write 工具（tokio::fs 写入文件）
- [x] 实现 grep 工具（文本搜索）
- [x] 实现 find 工具（文件查找，glob/ignore）
- [x] 实现 ls 工具（目录列表）
- [x] 实现 patch apply 策略（fudiff + patch fallback）
- [x] 实现 rtk 管道压缩集成（bash 工具可选压缩层，feature-gated）
- [x] 编写每个工具的单元测试（tempdir 隔离）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c20-add-tools --strict --no-interactive`
