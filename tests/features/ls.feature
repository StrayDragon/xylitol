# language: zh-CN
功能: Ls工具
  作为一个 LLM 代理
  我想要列出目录内容
  以便了解项目结构

  背景:
    假定 有一个临时工作目录

  场景: 列出空目录
    假定 存在空目录 "empty_dir"
    当 调用ls工具 路径 "empty_dir"
    那么 结果指示目录为空

  场景: 列出包含文件和子目录的目录
    假定 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在目录 "src/subdir"
    当 调用ls工具 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果包含 "subdir"

  场景: 条目按字母排序
    假定 目录 "sorted" 中存在文件 "z.txt" "a.txt" "m.txt"
    当 调用ls工具 路径 "sorted"
    那么 条目按字母顺序排列

  场景: 不传路径时默认当前目录
    假定 工作区根目录存在文件 "in_root.txt"
    当 调用ls 不传路径参数
    那么 结果列出 "in_root.txt"

  场景: 带限制参数的 ls
    假定 目录 "many" 中存在 100 个文件
    当 调用ls工具 路径 "many" 限制 10
    那么 恰好有 10 条结果
    并且 结果指示达到条目限制

  场景: 不存在的路径失败
    当 调用ls工具 路径 "/nonexistent/directory"
    那么 调用失败 包含错误信息

  场景: 路径指向文件而非目录失败
    假定 存在文件 "not_a_dir.txt"
    当 调用ls工具 路径 "not_a_dir.txt"
    那么 调用失败 包含错误信息 "Not a directory" 或 "不是目录"
