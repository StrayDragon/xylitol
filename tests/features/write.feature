# language: zh-CN
功能: 写入工具
  作为一个 LLM 代理
  我想要创建和覆写文件
  以便生成代码和配置

  背景:
    假定 有一个临时工作目录

  场景: 写入新文件
    当 调用write工具 路径 "src/output.txt" 内容 "hello world"
    那么 文件 "src/output.txt" 应该存在
    并且 文件 "src/output.txt" 内容为 "hello world"

  场景: 写入时自动创建父目录
    当 调用write工具 路径 "nested/deep/dir/file.txt" 内容 "深层内容"
    那么 文件 "nested/deep/dir/file.txt" 应该存在
    并且 文件 "nested/deep/dir/file.txt" 内容为 "深层内容"

  场景: 写入覆写已存在文件
    假定 存在文件 "src/existing.txt" 内容为 "旧内容"
    当 调用write工具 路径 "src/existing.txt" 内容 "新内容"
    那么 文件 "src/existing.txt" 内容为 "新内容"

  场景: 写入成功消息包含字节数
    当 调用write工具 路径 "src/size.txt" 内容 "hello"
    那么 结果包含 "success"

  场景: 缺少路径参数失败
    当 调用write 不传路径参数
    那么 调用失败 包含验证错误

  场景: 缺少内容参数失败
    当 调用write工具 路径 "test.txt" 不传内容
    那么 调用失败 包含验证错误
