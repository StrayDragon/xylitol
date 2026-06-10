# language: zh-CN
功能: 读取工具
  作为一个 LLM 代理
  我想要读取文件内容
  以便检查源代码和项目文件

  背景:
    假定 有一个临时工作目录

  场景: 读取整个文件
    假定 存在文件 "src/hello.rs" 内容为:
      """
      fn main() {
          println!("Hello, world!");
      }
      """
    当 调用read工具 路径 "src/hello.rs"
    那么 内容为:
      """
      fn main() {
          println!("Hello, world!");
      }
      """
    并且 总行数为 3

  场景: 读取文件带偏移和限制
    假定 存在文件 "src/multi.txt" 内容为:
      """
      第1行
      第2行
      第3行
      第4行
      第5行
      """
    当 调用read工具 路径 "src/multi.txt" 偏移 2 限制 2
    那么 内容为 "第2行\n第3行"
    并且 总行数为 5
    并且 偏移量为 2

  场景: 读取不存在的文件失败
    当 调用read工具 路径 "nonexistent.txt"
    那么 调用失败 包含错误信息 "not found" 或 "No such file" 或 "不存在"

  场景: 偏移超出文件末尾
    假定 存在文件 "src/short.txt" 内容为 "只有一行\n"
    当 调用read工具 路径 "src/short.txt" 偏移 10
    那么 内容为空
    并且 偏移量为 10

  场景: 输出超过限制时截断
    假定 存在文件 "src/large.txt" 包含10000行内容
    当 调用read工具 路径 "src/large.txt"
    那么 输出被截断
    并且 如果截断则显示剩余行提示

  场景: 缺少路径参数失败
    当 调用read 不传路径参数
    那么 调用失败 包含验证错误
