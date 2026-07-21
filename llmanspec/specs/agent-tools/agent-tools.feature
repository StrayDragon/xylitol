# language: zh-CN
# migrated from tests/features/{read,write,bash,edit,grep,find,ls}.feature
# scenario titles = English ids; docstring 已收成单行字符串（\n）
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

  场景: bash-timeout
    当 调用bash命令 "sleep 10" 超时 1 秒
    那么 命令应该失败 包含超时错误

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

  场景: edit-multi
    假定 存在文件 "src/lib.rs" 内容为 "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }"
    当 调用edit工具 路径 "src/lib.rs" 将 "a + b" 替换为 "a.wrapping_add(b)"
    并且 调用edit工具 路径 "src/lib.rs" 将 "a - b" 替换为 "a.wrapping_sub(b)"
    那么 文件 "src/lib.rs" 应该包含 "a.wrapping_add(b)"
    并且 文件 "src/lib.rs" 应该包含 "a.wrapping_sub(b)"

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

  场景: edit-crlf
    假定 存在文件 "src/windows.rs" 使用CRLF行尾 内容为 "// Windows 风格\n// 第二行"
    当 调用edit工具 路径 "src/windows.rs" 将 "Windows 风格" 替换为 "Unix 风格"
    那么 文件 "src/windows.rs" 应该包含 "Unix 风格"
    并且 文件 "src/windows.rs" 应该包含 "第二行"

  场景: edit-bom
    假定 存在文件 "src/bom.txt" 带UTF8_BOM 内容为 "Hello World"
    当 调用edit工具 路径 "src/bom.txt" 将 "Hello World" 替换为 "Hello BOM"
    那么 文件 "src/bom.txt" 应该保留UTF8_BOM
    并且 文件 "src/bom.txt" 应该包含 "Hello BOM"

  场景: edit-unicode
    假定 存在文件 "src/quotes.rs" 内容为 "let msg = \"hello world\";"
    当 调用edit工具 路径 "src/quotes.rs" 将 "let msg = \"hello world\";" 替换为 "let msg = \"hi earth\";"
    那么 文件 "src/quotes.rs" 应该包含 "hi earth"

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

  @req:r32
  场景: xy-tool-consistent
    假如 检查 protocol 工具边界
    当 应用变更后
    那么 所有工具边界类型均带 Xy 前缀

  @req:r42
  场景: happy
    假如 工具注册表含全部 7 个工具
    当 各工具以合法参数调用
    那么 各返回成功 ToolResult

  @req:r48
  场景: happy
    假如 AI 生成的 unified diff 略有行偏移
    当 经 fudiff 应用补丁
    那么 fudiff 在行偏移下仍成功应用

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
  场景: happy
    假如 工具需要必填字符串参数
    当 工具调用 require_str(args, 'file_path')
    那么 返回 Ok(value) 或含一致错误码的 Err(AdkError)

  @req:r10
  场景: happy
    假如 crate 在启用 dead_code lint 下编译
    当 运行 cargo clippy
    那么 生产路径无 dead_code 警告

  @req:r11
  场景: error-mapping
    假如 工具返回 XyToolError::InvalidArgs
    当 错误传播到 agent 循环
    那么 错误类别保留并展示给用户

  @req:t1
  场景: cancel
    假如 全部工具实现 XyTool
    当 触发取消
    那么 返回 abort 错误

  @req:t2
  场景: registry
    假如 调用 ToolRegistry builtins
    当 调用 list()
    那么 返回七个工具

  @req:t3
  场景: multi-edit
    假如 文件有 3 个不重叠区域
    当 调用 multi-edit
    那么 3 个区域均被替换

  @req:t4
  场景: overlap
    假如 两个重叠编辑
    当 调用 edit
    那么 错误含重叠及索引

  @req:t5
  场景: bom
    假如 文件含 UTF-8 BOM
    当 替换文本
    那么 BOM 保留且内容已更新

  @req:t6
  场景: diff
    假如 文件已编辑
    当 返回结果
    那么 含 unified patch 与带行号 diff

  @req:t8
  场景: find-basic
    假如 文件匹配 glob
    当 调用 find
    那么 fd 返回尊重 gitignore 的 Posix 路径

  @req:t9
  场景: bash-kill
    假如 长时间 bash 命令运行中
    当 CancellationToken 触发
    那么 进程被杀并返回 abort 错误

  @req:t10
  场景: read-trunc
    假如 存在 10000 行文件
    当 无限制调用 read
    那么 截断并附提示

  @req:t11
  场景: infra-works
    假如 infra 结构已定义
    当 全部工具使用它们
    那么 行为与 pi 完全一致

  @req:t12
  场景: toolset-unit-ops
    假如 构造 ToolSet
    当 链式 plus/remove/merge/retain
    那么 结果集仅反映单元操作且无运行时过滤状态

  @req:t13
  场景: bdd-pass
    假如 调用 BDD runner
    当 cargo test --test bdd
    那么 全部工具场景通过

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
    假如 bash 执行 echo hello
    当 经 OutputAccumulator 捕获输出
    那么 result.output 为 hello 且小输出时 result.full_output_path 为 None

  @req:t16
  场景: bdd-pass
    假如 调用 BDD runner
    当 cargo test --test bdd
    那么 全部 tool-v3 场景通过

  @req:e1
  场景: fuzzy
    假如 文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价
    当 应用 edit
    那么 模糊匹配成功并写入替换

  @req:e2
  场景: line-ending
    假如 文件使用 CRLF 或 LF
    当 应用 edit
    那么 输出文件保留原行尾风格

  @req:e3
  场景: span
    假如 oldText 跨 5 行但中间一行精确匹配失败
    当 以 span 匹配应用 edit
    那么 滑动窗口匹配成功

  @req:e4
  场景: diff
    假如 edit 应用变更
    当 计算 diff
    那么 结果含变更标记的前后行

  @req:t17
  场景: tool-def-contains-schema
    假如 构造 ToolDefinition
    当 应用变更后
    那么 存储 schema: XyToolSchema 且 prompt 元数据分离

  @req:t18
  场景: toolset-in-agent-final
    假如 定位工具容器类型
    当 检查模块路径
    那么 为 agent::tools 的 ToolSet，一轮接收最终集合

  @req:t19
  场景: tool-mode-renamed
    假如 代码引用 ToolExecutionMode
    当 应用变更后
    那么 各处重命名为 XyToolExecutionMode

  @req:t20
  场景: loop-uses-final-set
    假如 检查循环源码
    当 应用变更后
    那么 无 set_allowed 或 list_filtered 符号且循环直接迭代 ToolSet

  @req:r36
  场景: via-port
    当 rg rmcp 于 src/agent 与 src/protocol
    那么 零匹配

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
