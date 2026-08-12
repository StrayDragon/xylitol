# language: zh-CN
# migrated from tests/features/{read,write,bash,edit,grep,find,ls}.feature
# scenario titles = English ids; docstring 已收成单行字符串（\n）
# BDD 接线（tests/bdd.rs）：七工具主路径已绑，含 read-truncate / bash-timeout /
# edit-empty-old / edit-nonunique / edit-noop（无 edit-oldtext-* 旧 id）
功能: agent-tools
  背景:
    假定 有一个临时工作目录
    并且 存在文件 "src/main.rs" 内容为 "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}"

  场景: read-entire
    假定 存在文件 "src/hello.rs" 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
    当 调用read工具 路径 "src/hello.rs"
    那么 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
    并且 总行数为 3

  场景: read-offset-limit
    假定 存在文件 "src/multi.txt" 内容为 "第1行\n第2行\n第3行\n第4行\n第5行"
    当 调用read工具 路径 "src/multi.txt" 偏移 2 限制 2
    那么 内容为 "第2行\n第3行"
    并且 总行数为 5
    并且 偏移量为 2

  场景: read-missing
    当 调用read工具 路径 "nonexistent.txt"
    那么 调用失败 包含错误信息 "not found" 或 "No such file" 或 "不存在"

  场景: read-offset-oob
    假定 存在文件 "src/short.txt" 内容为 "只有一行\n"
    当 调用read工具 路径 "src/short.txt" 偏移 10
    那么 内容为空
    并且 偏移量为 10

  @req:t10
  场景: read-truncate
    假定 存在文件 "src/large.txt" 包含10000行内容
    当 调用read工具 路径 "src/large.txt"
    那么 输出被截断
    并且 如果截断则显示剩余行提示

  场景: read-missing-path
    当 调用read 不传路径参数
    那么 调用失败 包含验证错误

  场景: write-new
    当 调用write工具 路径 "src/output.txt" 内容 "hello world"
    那么 文件 "src/output.txt" 应该存在
    并且 文件 "src/output.txt" 内容为 "hello world"

  场景: write-parents
    当 调用write工具 路径 "nested/deep/dir/file.txt" 内容 "深层内容"
    那么 文件 "nested/deep/dir/file.txt" 应该存在
    并且 文件 "nested/deep/dir/file.txt" 内容为 "深层内容"

  场景: write-overwrite
    假定 存在文件 "src/existing.txt" 内容为 "旧内容"
    当 调用write工具 路径 "src/existing.txt" 内容 "新内容"
    那么 文件 "src/existing.txt" 内容为 "新内容"

  场景: write-byte-count
    当 调用write工具 路径 "src/size.txt" 内容 "hello"
    那么 结果包含 "success"

  场景: write-missing-path
    当 调用write 不传路径参数
    那么 调用失败 包含验证错误

  场景: write-missing-content
    当 调用write工具 路径 "test.txt" 不传内容
    那么 调用失败 包含验证错误

  场景: bash-echo
    当 调用bash命令 "echo hello world"
    那么 退出码为 0
    并且 stdout 包含 "hello world"

  场景: bash-stderr
    当 调用bash命令 "echo 错误信息 >&2"
    那么 stdout 和 stderr 合并输出包含 "错误信息"

  场景: bash-exit-code
    当 调用bash命令 "exit 42"
    那么 退出码为 42

  @req:t24
  场景: bash-timeout
    当 调用bash命令 "sleep 10" 超时 1 秒
    那么 命令应该失败 包含超时错误

  @req:t24
  场景: bash-omit-timeout-completes
    当 调用bash命令 "sleep 2"
    那么 退出码为 0

  场景: bash-merged-streams
    当 调用bash命令 "echo out && echo err >&2 && echo out2"
    那么 stdout 和 stderr 合并输出包含 "out"
    并且 stdout 和 stderr 合并输出包含 "err"
    并且 stdout 和 stderr 合并输出包含 "out2"

  场景: bash-truncate
    当 调用bash命令 "yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx | head -n 3000"
    那么 输出被截断
    并且 截断详情显示达到字节或行限制
    并且 结果含 Full output 脚注
    并且 bash 结果 JSON 无未截断全量 stdout 字段载荷

  @req:t23
  场景: bash-result-no-full-dump
    当 调用bash命令 "yes yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy | head -n 3000"
    那么 结果含 Full output 脚注
    并且 bash 结果 JSON 无未截断全量 stdout 字段载荷

  @req:t9
  场景: bash-cancel
    当 调用bash命令 "sleep 60"
    并且 在500ms后发送取消信号
    那么 命令应该失败 包含取消错误

  场景: bash-missing-cmd
    当 调用bash 不传命令参数
    那么 调用失败 包含验证错误

  场景: edit-single
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"你好\");"
    那么 文件 "src/main.rs" 应该包含 "你好"

  @req:t3
  场景: edit-multi
    假定 存在文件 "src/lib.rs" 内容为 "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }"
    当 调用edit工具 路径 "src/lib.rs" 将 "a + b" 替换为 "a.wrapping_add(b)"
    并且 调用edit工具 路径 "src/lib.rs" 将 "a - b" 替换为 "a.wrapping_sub(b)"
    那么 文件 "src/lib.rs" 应该包含 "a.wrapping_add(b)"
    并且 文件 "src/lib.rs" 应该包含 "a.wrapping_sub(b)"

  @req:t4
  场景: edit-overlap
    假定 存在文件 "src/overlap.rs" 内容为 "fn hello_world() {\n    println!(\"hello world\");\n}"
    当 调用edit工具 路径 "src/overlap.rs" 做重叠替换

  场景: edit-nonunique
    假定 存在文件 "src/dup.rs" 内容为 "let x = 1;\nlet x = 2;"
    当 调用edit工具 路径 "src/dup.rs" 做重复替换
    那么 edit调用应该失败 包含错误信息 "not unique"

  场景: edit-empty-old
    当 调用edit工具 路径 "src/main.rs" 将 "" 替换为 "foo"
    那么 edit调用应该失败 包含错误信息 "empty" 或 "空"

  场景: edit-noop
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"hello\");"
    那么 edit调用应该失败 包含错误信息 "identical" 或 "No changes" 或 "no change" 或 "not unique"

  @req:e2
  场景: edit-crlf
    假定 存在文件 "src/windows.rs" 使用CRLF行尾 内容为 "// Windows 风格\n// 第二行"
    当 调用edit工具 路径 "src/windows.rs" 将 "Windows 风格" 替换为 "Unix 风格"
    那么 文件 "src/windows.rs" 应该包含 "Unix 风格"
    并且 文件 "src/windows.rs" 应该包含 "第二行"

  @req:t5
  场景: edit-bom
    假定 存在文件 "src/bom.txt" 带UTF8_BOM 内容为 "Hello World"
    当 调用edit工具 路径 "src/bom.txt" 将 "Hello World" 替换为 "Hello BOM"
    那么 文件 "src/bom.txt" 应该保留UTF8_BOM
    并且 文件 "src/bom.txt" 应该包含 "Hello BOM"

  场景: edit-unicode
    假定 存在文件 "src/quotes.rs" 内容为 "let msg = \"hello world\";"
    当 调用edit工具 路径 "src/quotes.rs" 将 "let msg = \"hello world\";" 替换为 "let msg = \"hi earth\";"
    那么 文件 "src/quotes.rs" 应该包含 "hi earth"

  @req:t6
  @req:e4
  场景: edit-diff
    假定 存在文件 "src/diff_test.rs" 内容为 "第1行\n第2行\n第3行"
    当 调用edit工具 路径 "src/diff_test.rs" 将 "第2行" 替换为 "第二行"
    那么 结果包含 "@@"

  @req:t7
  场景: grep-basic
    假定 存在文件 "src/data.txt" 内容为 "apple\nbanana\ncherry\napple pie\norange"
    当 调用grep 模式 "apple" 路径 "src/data.txt"
    那么 匹配结果包含第1行的 "apple"
    并且 匹配结果包含第4行的 "apple pie"
    并且 共有 2 条匹配

  场景: grep-no-match
    假定 存在文件 "src/data.txt" 内容为 "foo\nbar\nbaz"
    当 调用grep 模式 "nonexistent" 路径 "src/data.txt"
    那么 结果应该为空或提示无匹配

  场景: grep-limit
    假定 存在文件 "src/many.txt" 包含20行 "match"
    当 调用grep 模式 "match" 路径 "src/many.txt" 限制 5
    那么 恰好有 5 条匹配

  场景: grep-ignore-case
    假定 存在文件 "src/case.txt" 内容为 "Hello World\nHELLO WORLD\nhello world"
    当 调用grep 不区分大小写 模式 "hello" 路径 "src/case.txt"
    那么 共有 3 条匹配

  场景: grep-literal
    假定 存在文件 "src/literal.txt" 内容为 "function(x)\nfunction(y)\nfn.call()"
    当 调用grep 字面量模式 "fn.call()" 路径 "src/literal.txt"
    那么 结果包含 "fn.call()"

  场景: grep-missing-pattern
    当 调用grep 不传模式参数
    那么 调用失败 包含验证错误

  @req:t8
  场景: find-simple
    假定 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在文件 "src/main.txt"
    当 调用find 模式 "*.rs" 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果不包含 "main.txt"

  场景: find-recursive
    假定 存在文件 "src/lib.rs"
    并且 存在文件 "src/sub/mod.rs"
    并且 存在文件 "tests/test.rs"
    当 调用find 模式 "**/*.rs" 路径 "."
    那么 结果包含 "src/lib.rs"
    并且 结果包含 "src/sub/mod.rs"
    并且 结果包含 "tests/test.rs"

  场景: find-limit
    假定 存在 50 个文件匹配模式
    当 调用find 模式 "*.log" 路径 "." 限制 10
    那么 恰好有 10 条结果

  场景: find-no-match
    当 调用find 模式 "*.nonexistent" 路径 "."
    那么 结果包含 "No files found" 或 "未找到文件"

  场景: find-bad-path
    当 调用find 模式 "*.rs" 路径 "/nonexistent/path"
    那么 调用失败 包含错误信息

  场景: find-absolute
    当 调用find工具 查找绝对路径失败
    那么 调用失败 包含错误信息

  场景: ls-empty
    假定 存在空目录 "empty_dir"
    当 调用ls工具 路径 "empty_dir"
    那么 结果指示目录为空

  场景: ls-entries
    假定 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在目录 "src/subdir"
    当 调用ls工具 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果包含 "subdir"

  场景: ls-sorted
    假定 目录 "sorted" 中存在文件 "z.txt" "a.txt" "m.txt"
    当 调用ls工具 路径 "sorted"
    那么 条目按字母顺序排列

  场景: ls-default-cwd
    假定 工作区根目录存在文件 "in_root.txt"
    当 调用ls 不传路径参数
    那么 结果列出 "in_root.txt"

  场景: ls-limit
    假定 目录 "many" 中存在 100 个文件
    当 调用ls工具 路径 "many" 限制 10
    那么 恰好有 10 条结果
    并且 结果指示达到条目限制

  场景: ls-missing
    当 调用ls工具 路径 "no_such_dir"
    那么 调用失败 包含错误信息

  场景: ls-not-dir
    假定 存在文件 "src/main.rs"
    当 调用ls工具 路径 "src/main.rs"
    那么 调用失败 包含错误信息

  @req:r42
  场景: all-ten-tools-smoke
    假如 工具注册表含全部 10 个工具
    当 各工具以合法参数调用
    那么 各返回成功 ToolResult

  @req:r48
  场景: fudiff-line-offset
    假如 临时目录存在含行偏移的 unified diff 文件
    当 经补丁应用执行 edit
    那么 模糊匹配补丁应用成功

  @req:r4
  场景: large-file
    假如 存在 50MB 文件
    当 read 工具在无 offset/limit 时调用
    那么 工具返回文件过大错误

  @req:r5
  场景: output-overflow
    假如 bash 运行 yes 命令
    当 stdout 超过 1MB 上限
    那么 子进程被杀并返回截断输出

  @req:r6
  场景: negative-timeout
    假如 LLM 传入 timeout=-1
    当 bash 工具校验参数
    那么 工具以无效 timeout 错误拒绝

  @req:r6
  场景: zero-timeout-rejected
    假如 LLM 传入 timeout=0
    当 bash 工具校验参数
    那么 工具以无效 timeout 错误拒绝

  @req:r7
  场景: absolute-pattern
    假如 find 以 pattern='/etc/**' 调用
    当 安全已启用
    那么 工具返回错误或过滤结果至 root

  @req:r8
  场景: multibyte-truncation
    假如 bash 输出在上限边界以不完整 UTF-8 序列结束
    当 调用 truncate_output
    那么 输出在字符边界安全截断且不 panic

  @req:r9
  场景: require-str-missing-arg
    假如 工具需要必填字符串参数 file_path
    当 以空参调用该工具
    那么 调用失败且返回 MissingArgument 错误码

  @req:r11
  场景: error-mapping
    假如 工具以 InvalidArgs 错误执行失败
    当 agent 循环收集工具结果
    那么 产生的 AgentEvent 含 error 类别

  @req:t1
  场景: cancel
    假如 bash 工具正在 sleep 60
    当 发送取消信号
    那么 execute 返回 Cancelled 错误

  @req:t2
  场景: registry
    假如 工具集含全部内置工具
    当 列举工具名
    那么 返回 10 个工具名

  @req:t11
  场景: infra-works
    假如 内置工具集已构造
    当 分别执行 read / write / edit / bash / grep / find / ls
    那么 各工具返回成功结果且无 panic

  @req:t12
  场景: toolset-unit-ops
    假如 工具集含 read 与 grep
    当 plus(bash) 然后 remove(grep)
    那么 最终工具集含 read 与 bash 且不含 grep

  @req:t14
  场景: small-output
    假如 累加器接收 100 字节
    当 调用 finish
    那么 snapshot.content 含 100 字节，未创建临时文件

  @req:t14
  场景: overflow-temp-file
    假如 累加器接收 2x max_bytes
    当 调用 finish
    那么 快照内容为截断尾部，full_output_path 指向含完整输出的临时文件

  @req:t15
  场景: bash-uses-accumulator
    假如 bash 工具即将执行 echo hello
    当 执行 bash echo hello
    那么 返回 output:hello 且未触发临时文件落盘

  @req:e1
  场景: fuzzy
    假如 文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价
    当 应用 edit
    那么 模糊匹配成功并写入替换

  @req:e3
  场景: span
    假如 oldText 跨 5 行但中间一行精确匹配失败
    当 以 span 匹配应用 edit
    那么 滑动窗口匹配成功

  @req:t21
  场景: png-yields-image-part
    假如 临时目录有小 PNG
    当 调用 read
    那么 tool result parts 含 Image 且 data 非空

  @req:t21
  场景: text-still-works
    假如 临时目录有 .txt
    当 调用 read
    那么 返回文本内容且无 Image part
