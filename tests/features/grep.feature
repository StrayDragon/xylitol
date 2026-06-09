# language: zh-CN
功能: Grep工具
  作为一个 LLM 代理
  我想要在文件内容中搜索模式
  以便发现代码和理解代码库

  背景:
    假定 有一个临时工作目录

  场景: 文件中基本模式搜索
    假定 存在文件 "src/data.txt" 内容为:
      """
      apple
      banana
      cherry
      apple pie
      orange
      """
    当 调用grep 模式 "apple" 路径 "src/data.txt"
    那么 匹配结果包含第1行的 "apple"
    并且 匹配结果包含第4行的 "apple pie"
    并且 共有 2 条匹配

  场景: 无匹配返回适当消息
    假定 存在文件 "src/data.txt" 内容为:
      """
      foo
      bar
      baz
      """
    当 调用grep 模式 "nonexistent" 路径 "src/data.txt"
    那么 结果包含 "No matches found" 或 "未找到匹配"

  场景: 搜索遵守限制参数
    假定 存在文件 "src/many.txt" 包含20行 "match"
    当 调用grep 模式 "match" 路径 "src/many.txt" 限制 5
    那么 恰好有 5 条匹配

  场景: 不区分大小写搜索
    假定 存在文件 "src/case.txt" 内容为:
      """
      Hello World
      HELLO WORLD
      hello world
      """
    当 调用grep 不区分大小写 模式 "hello" 路径 "src/case.txt"
    那么 共有 3 条匹配

  场景: 字面量字符串搜索
    假定 存在文件 "src/literal.txt" 内容为:
      """
      function(x)
      function(y)
      fn.call()
      """
    当 调用grep 字面量模式 "fn.call()" 路径 "src/literal.txt"
    那么 结果包含 "fn.call()"

  场景: 缺少模式参数失败
    当 调用grep 不传模式参数
    那么 调用失败 包含验证错误
