# language: zh-CN
功能: 编辑工具
  作为一个 LLM 代理
  我想要通过精确文本替换来编辑文件
  以便进行精准的非重叠修改

  背景:
    假定 有一个临时工作目录
    并且 存在文件 "src/main.rs" 内容为:
      """
      fn main() {
          println!("hello");
          println!("world");
      }
      """

  场景: 单次精确文本替换
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"你好\");"
    那么 文件 "src/main.rs" 应该包含 "你好"

  场景: 一次调用中多个不相交的编辑
    假定 存在文件 "src/lib.rs" 内容为:
      """
      pub fn add(a: i32, b: i32) -> i32 { a + b }
      pub fn sub(a: i32, b: i32) -> i32 { a - b }
      """
    当 调用edit工具 路径 "src/lib.rs" 进行2处替换:
      | a + b | a.wrapping_add(b) |
      | a - b | a.wrapping_sub(b) |
    那么 文件 "src/lib.rs" 应该包含 "a.wrapping_add(b)"
    并且 文件 "src/lib.rs" 应该包含 "a.wrapping_sub(b)"

  场景: 重叠编辑被拒绝
    假定 存在文件 "src/overlap.rs" 内容为:
      """
      fn hello_world() {
          println!("hello world");
      }
      """
    当 调用edit工具 路径 "src/overlap.rs" 进行2处替换:
      | fn hello_world() {\n    println!("hello world");\n} | fn greet() {\n    println!("hi");\n} |
      | println!("hello world") | println!("hi") |
    那么 edit调用应该失败 包含错误信息 "overlap" 或 "重叠"

  场景: 非唯一的 oldText 被拒绝
    假定 存在文件 "src/dup.rs" 内容为:
      """
      let x = 1;
      let x = 2;
      """
    当 调用edit工具 路径 "src/dup.rs" 将 "let x" 替换为 "let y"
    那么 edit调用应该失败 包含错误信息 "unique" 或 "occurrences" 或 "唯一"

  场景: 空的 oldText 被拒绝
    当 调用edit工具 路径 "src/main.rs" 将 "" 替换为 "foo"
    那么 edit调用应该失败 包含错误信息 "empty" 或 "空"

  场景: 无变更的编辑被拒绝
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"hello\");"
    那么 edit调用应该失败 包含错误信息 "identical" 或 "No changes" 或 "未变更"

  场景: 编辑保留 CRLF 行尾
    假定 存在文件 "src/windows.rs" 使用CRLF行尾 内容为:
      """
      // Windows 风格
      // 第二行
      """
    当 调用edit工具 路径 "src/windows.rs" 将 "Windows 风格" 替换为 "Unix 风格"
    那么 文件 "src/windows.rs" 应该保持CRLF行尾
    并且 文件 "src/windows.rs" 应该包含 "Unix 风格"

  场景: 编辑处理 UTF-8 BOM
    假定 存在文件 "src/bom.txt" 带UTF8_BOM 内容为 "Hello World"
    当 调用edit工具 路径 "src/bom.txt" 将 "Hello World" 替换为 "Hello BOM"
    那么 文件 "src/bom.txt" 应该保留UTF8_BOM
    并且 文件 "src/bom.txt" 应该包含 "Hello BOM"

  场景: 模糊Unicode匹配
    假定 存在文件 "src/quotes.rs" 内容为:
      """
      let msg = "hello world";
      """
    当 调用edit工具 路径 "src/quotes.rs" 将 "let msg = \u201Chello world\u201D;" 替换为 "let msg = \"hi earth\";"
    那么 文件 "src/quotes.rs" 应该包含 "let msg = \"hi earth\";"

  场景: 编辑返回 unified diff
    假定 存在文件 "src/diff_test.rs" 内容为:
      """
      第1行
      第2行
      第3行
      """
    当 调用edit工具 路径 "src/diff_test.rs" 将 "第2行" 替换为 "第二行"
    那么 结果包含 unified patch
    并且 结果包含 带行号的 display diff
