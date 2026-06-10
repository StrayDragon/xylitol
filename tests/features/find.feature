# language: zh-CN
功能: Find工具
  作为一个 LLM 代理
  我想要通过 glob 模式查找文件
  以便发现项目结构和定位文件

  背景:
    假定 有一个临时工作目录

  场景: 通过简单 glob 查找文件
    假定 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在文件 "src/main.txt"
    当 调用find 模式 "*.rs" 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果不包含 "main.txt"

  场景: 递归 glob 查找
    假定 存在文件 "src/lib.rs"
    并且 存在文件 "src/sub/mod.rs"
    并且 存在文件 "tests/test.rs"
    当 调用find 模式 "**/*.rs" 路径 "."
    那么 结果包含 "src/lib.rs"
    并且 结果包含 "src/sub/mod.rs"
    并且 结果包含 "tests/test.rs"

  场景: 查找带限制参数
    假定 存在 50 个文件匹配模式
    当 调用find 模式 "*.log" 路径 "." 限制 10
    那么 恰好有 10 条结果

  场景: 无匹配返回适当消息
    当 调用find 模式 "*.nonexistent" 路径 "."
    那么 结果包含 "No files found" 或 "未找到文件"

  场景: 不存在的搜索路径失败
    当 调用find 模式 "*.rs" 路径 "/nonexistent/path"
    那么 调用失败 包含错误信息

  场景: 绝对路径 glob 被拒绝
    当 调用find工具 查找绝对路径失败
