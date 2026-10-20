# language: zh-CN
# purpose: 工具系统 — 工具定义、执行分发、文件类内置与 Todo 两工具、路径工具与并发类。

# ── 文件导览（43 规则 / 86 场景）──────────────────────────────
#   内置闭集与工具框架 : 默认内置工具闭集 · ToolSet · 工具错误类型 · 参数/schema
#   文件与编辑         : read · write · edit（多段/校验/Unicode/diff）· image
#   检索               : grep ripgrep · find（禁止绝对路径逃逸）
#   bash/shell         : 流式输出 · 超时 · 中止 · 累加器 · fs 有界
#   补丁/diff 应用     : 补丁应用 · 模糊匹配 · 行尾 · 跨行 span · diff 输出
#   并发/屏障          : builtin concurrency-class · dynamic-tool MCP hard barrier
#   （精确检索以 @req/规则标题为准，本导览为避免 500+ 行无导航而作，非强制分组）
# ────────────────────────────────────────────────────────────────

功能: agent-tools
  背景:
    假如 有一个临时工作目录
    并且 存在文件 "src/main.rs" 内容为 "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}"

  @req:r32
  规则: 工具 trait 前缀
    System 须将主工具接口定义为 XyTool；trait 名已带 Xy 前缀，须与 protocol 边界其余命名一致。
    场景: 内置名称列出经XyTool
      假如 工具集含全部内置工具
      当 列举工具名
      那么 返回 9 个工具名

  @req:r42
  规则: 默认内置工具闭集
    产品 default_tools（Print 与 TUI builtins 基座）须包含且以闭集提供：read、bash、edit、write、grep、find、ls、todo_rewrite、todo_update（共 9 个）。ask 等面专属工具 不得计入该闭集（见 t28）。Todo 工具语义见 agent-todo。不得向模型暴露 todo_list。

    场景: 全部十工具冒烟
      假如 工具注册表含全部 9 个工具
      当 各工具以合法参数调用
      那么 各返回成功 ToolResult
  @req:r48
  规则: 补丁应用
    System 须使用 fudiff 模糊匹配应用 AI 生成的补丁，并以 patch crate 精确匹配作为回退。

    场景: fudiff行偏移
      假如 临时目录存在含行偏移的 unified diff 文件
      当 经补丁应用执行 edit
      那么 模糊匹配补丁应用成功
  @req:r4
  规则: 读文件大小限制
    Read 工具 须拒绝超过 MAX_FILE_SIZE（默认 10MB）的文件，并返回清晰错误信息。

    场景: 大文件
      假如 存在 50MB 文件
      当 read 工具在无 offset/limit 时调用
      那么 工具返回文件过大错误
  @req:r5
  规则: bash 流式输出
    Bash 工具 须流式输出 stdout/stderr 并设字节上限；超限时 须终止子进程。

    场景: bash截断
      当 调用bash命令 "yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx | head -n 3000"
      那么 输出被截断
      并且 截断详情显示达到字节或行限制
      并且 结果含 Full output 脚注
      并且 bash 结果 JSON 无未截断全量 stdout 字段载荷

    场景: 输出溢出
      假如 bash 运行 yes 命令
      当 stdout 超过 1MB 上限
      那么 子进程被杀并返回截断输出
  @req:r6
  规则: 整数转换校验
    工具 须在转换前校验数值参数（timeout、max_results）处于正数安全范围内；timeout 若出现则 须为正整数秒，超过全局上界的值 须被程序钳制到上界而非拒绝，不得将 0 解释为无限。

    场景: 负数超时
      假如 LLM 传入 timeout=-1
      当 bash 工具校验参数
      那么 工具以无效 timeout 错误拒绝

    场景: 零超时拒绝
      假如 LLM 传入 timeout=0
      当 bash 工具校验参数
      那么 工具以无效 timeout 错误拒绝
  @req:r1148
  规则: 工具超时默认有界
    命令类工具（bash、grep、find）的执行等待由程序最高权限管理：timeout 为模型侧请求——正整数秒被接受并钳制到全局上界（600 秒），省略时按工具默认（bash 120 秒、grep/find 60 秒）；计时器 须必然武装，到期终止并返回 timeout 类错误；不得存在无限语义或可绕过计时的输入。任何外部等待 不得超过全局兜底上界（600 秒）。流式与非流式路径共用同一语义。

    场景: bash超时
      当 调用bash命令 "sleep 10" 超时 1 秒
      那么 命令应该失败 包含超时错误

    场景: bash省略超时完成
      当 调用bash命令 "sleep 2"
      那么 退出码为 0
  @req:r1149
  规则: grep-find-timeout-schema
    grep 与 find 的 parameters schema 须暴露可选 timeout（秒）；行为遵循 t24。

    场景: 搜索工具暴露可选超时
      当 检查内置工具的参数 schema
      那么 grep 与 find 的 schema 含可选 timeout
  @req:r1153
  规则: fs 工具有界
    read、write、edit、ls 四个文件类工具的执行等待 须有默认上限（30 秒），到期以 timeout 类错误呈现；不得向模型暴露 per-call timeout 参数。

    场景: 文件系统工具隐藏每调用超时
      当 检查内置工具的参数 schema
      那么 文件类四工具的 schema 不含 per-call timeout 参数
  @req:r7
  规则: find 禁止绝对路径逃逸
    Find 工具 须拒绝绝对 pattern，或 须规范化结果以验证其仍在 root 目录内。

    场景: 查找绝对
      当 调用find工具 查找绝对路径失败
      那么 调用失败 包含错误信息

    场景: 绝对模式
      假如 find 以 pattern='/etc/**' 调用
      当 安全已启用
      那么 工具返回错误或过滤结果至 root
  @req:r8
  规则: 安全 UTF-8 截断
    输出截断 须使用字符边界感知切片，防止 UTF-8 panic。

    场景: 多字节截断
      假如 bash 输出在上限边界以不完整 UTF-8 序列结束
      当 调用 truncate_output
      那么 输出在字符边界安全截断且不 panic
  @req:r9
  规则: 工具参数辅助函数
    工具系统 须提供共享参数提取辅助函数，消除各工具实现中重复的 JSON 字段解析。

    场景: 需要字符串缺失参数
      假如 工具需要必填字符串参数 file_path
      当 以空参调用该工具
      那么 调用失败且返回 MissingArgument 错误码
  @req:r10
  规则: 禁止全局 dead_code 允许
    crate 根 不得使用全局 #![allow(dead_code)]；dead code 抑制 须限定于单项并附理由。
    场景: 失效代码允许为项限定
      当 读取 crate 根的死代码允许写法
      那么 无全局 allow 且单项抑制带理由

  @req:r11
  规则: 工具错误类型
    System 须定义 XyToolError，含结构化错误类别（invalid_args / execution_failed / permission_denied / timeout），独立于 adk-core。

    场景: 读取缺失
      当 调用read工具 路径 "nonexistent.txt"
      那么 调用失败 包含错误信息 "not found" 或 "No such file" 或 "不存在"

    场景: 读取缺失路径
      当 调用read 不传路径参数
      那么 调用失败 包含验证错误

    场景: 写入缺失路径
      当 调用write 不传路径参数
      那么 调用失败 包含验证错误

    场景: 写入缺失内容
      当 调用write工具 路径 "test.txt" 不传内容
      那么 调用失败 包含验证错误

    场景: bash缺失命令
      当 调用bash 不传命令参数
      那么 调用失败 包含验证错误

    场景: grep缺失模式
      当 调用grep 不传模式参数
      那么 调用失败 包含验证错误

    场景: 查找坏路径
      当 调用find 模式 "*.rs" 路径 "/nonexistent/path"
      那么 调用失败 包含错误信息

    场景: ls缺失
      当 调用ls工具 路径 "no_such_dir"
      那么 调用失败 包含错误信息

    场景: ls不目录
      假如 存在文件 "src/main.rs"
      当 调用ls工具 路径 "src/main.rs"
      那么 调用失败 包含错误信息

    场景: 错误映射
      假如 工具以 InvalidArgs 错误执行失败
      当 agent 循环收集工具结果
      那么 产生的 AgentEvent 含 error 类别
  @req:r1135
  规则: traits
    System 须定义 XyTool trait，execute 接受 CancellationToken。

    场景: 取消
      假如 bash 工具正在 sleep 60
      当 发送取消信号
      那么 execute 返回 Cancelled 错误
  @req:r1144
  规则: 默认内置工具闭集·t2
    System 须实现与 r42 相同的 9 个 default_tools 闭集成员：read、bash、edit、write、grep、find、ls、todo_rewrite、todo_update。文件类七工具行为对齐既有 pi 等价语义；Todo 两工具行为见 agent-todo。

    场景: 读取整段
      假如 存在文件 "src/hello.rs" 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
      当 调用read工具 路径 "src/hello.rs"
      那么 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
      并且 总行数为 3

    场景: 写入新
      当 调用write工具 路径 "src/output.txt" 内容 "hello world"
      那么 文件 "src/output.txt" 应该存在
      并且 文件 "src/output.txt" 内容为 "hello world"

    场景: 写入父级
      当 调用write工具 路径 "nested/deep/dir/file.txt" 内容 "深层内容"
      那么 文件 "nested/deep/dir/file.txt" 应该存在
      并且 文件 "nested/deep/dir/file.txt" 内容为 "深层内容"

    场景: 写入覆写
      假如 存在文件 "src/existing.txt" 内容为 "旧内容"
      当 调用write工具 路径 "src/existing.txt" 内容 "新内容"
      那么 文件 "src/existing.txt" 内容为 "新内容"

    场景: bash回显
      当 调用bash命令 "echo hello world"
      那么 退出码为 0
      并且 stdout 包含 "hello world"

    场景: bash退出代码
      当 调用bash命令 "exit 42"
      那么 退出码为 42

    场景: ls空
      假如 存在空目录 "empty_dir"
      当 调用ls工具 路径 "empty_dir"
      那么 结果指示目录为空

    场景: ls条目
      假如 存在文件 "src/main.rs"
      并且 存在文件 "src/lib.rs"
      并且 存在目录 "src/subdir"
      当 调用ls工具 路径 "src"
      那么 结果包含 "main.rs"
      并且 结果包含 "lib.rs"
      并且 结果包含 "subdir"

    场景: ls已排序
      假如 目录 "sorted" 中存在文件 "z.txt" "a.txt" "m.txt"
      当 调用ls工具 路径 "sorted"
      那么 条目按字母顺序排列

    场景: ls上限
      假如 目录 "many" 中存在 100 个文件
      当 调用ls工具 路径 "many" 限制 10
      那么 恰好有 10 条结果
      并且 结果指示达到条目限制

    场景: 注册表
      假如 工具集含全部内置工具
      当 列举工具名
      那么 返回 9 个工具名
  @req:r1154
  规则: edit 多段编辑
    Edit 工具 须接受 path 与 edits 数组（oldText、newText），对照原始内容匹配。

    场景: 编辑单一
      当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"你好\");"
      那么 文件 "src/main.rs" 应该包含 "你好"

    场景: 编辑多
      假如 存在文件 "src/lib.rs" 内容为 "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }"
      当 调用edit工具 路径 "src/lib.rs" 将 "a + b" 替换为 "a.wrapping_add(b)"
      并且 调用edit工具 路径 "src/lib.rs" 将 "a - b" 替换为 "a.wrapping_sub(b)"
      那么 文件 "src/lib.rs" 应该包含 "a.wrapping_add(b)"
      并且 文件 "src/lib.rs" 应该包含 "a.wrapping_sub(b)"
  @req:r1155
  规则: edit 校验
    Edit 工具 须拒绝重叠编辑、非唯一 oldText、空 oldText 与无变化编辑。

    场景: 编辑重叠
      假如 存在文件 "src/overlap.rs" 内容为 "fn hello_world() {\n    println!(\"hello world\");\n}"
      当 调用edit工具 路径 "src/overlap.rs" 做重叠替换
      那么 edit调用应该失败 包含错误信息 "Overlapping edits"

    场景: 编辑非唯一
      假如 存在文件 "src/dup.rs" 内容为 "let x = 1;\nlet x = 2;"
      当 调用edit工具 路径 "src/dup.rs" 做重复替换
      那么 edit调用应该失败 包含错误信息 "not unique"

    场景: 编辑空旧
      当 调用edit工具 路径 "src/main.rs" 将 "" 替换为 "foo"
      那么 edit调用应该失败 包含错误信息 "empty" 或 "空"

    场景: 编辑空操作
      当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"hello\");"
      那么 edit调用应该失败 包含错误信息 "identical" 或 "No changes" 或 "no change" 或 "not unique"
  @req:r1156
  规则: edit Unicode
    Edit 工具 须将 CRLF 规范为 LF、剥离 UTF-8 BOM，并通过 NFKC 规范化做模糊匹配。

    场景: 编辑BOM
      假如 存在文件 "src/bom.txt" 带UTF8_BOM 内容为 "Hello World"
      当 调用edit工具 路径 "src/bom.txt" 将 "Hello World" 替换为 "Hello BOM"
      那么 文件 "src/bom.txt" 应该保留UTF8_BOM
      并且 文件 "src/bom.txt" 应该包含 "Hello BOM"
  @req:r1157
  规则: edit diff
    Edit 工具 须对编辑前后的整文件内容产生 unified patch 与带行号的展示 diff（display_diff 须为 context hunk，不得把全文件 Equal 行展开进展示；展示行数 须有硬上限并在超限时标注截断）；发往模型的 tool result 正文 不得嵌入完整 diff/display_diff。

    场景: 编辑diff
      假如 存在文件 "src/diff_test.rs" 内容为 "第1行\n第2行\n第3行"
      当 调用edit工具 路径 "src/diff_test.rs" 将 "第2行" 替换为 "第二行"
      那么 结果包含 "@@"
  @req:r1158
  规则: grep ripgrep
    Grep 工具 须使用 ripgrep，支持 regex、glob、ignoreCase、literal、context、limit。

    场景: grep基础
      假如 存在文件 "src/data.txt" 内容为 "apple\nbanana\ncherry\napple pie\norange"
      当 调用grep 模式 "apple" 路径 "src/data.txt"
      那么 匹配结果包含第1行的 "apple"
      并且 匹配结果包含第4行的 "apple pie"
      并且 共有 2 条匹配

    场景: grep无匹配
      假如 存在文件 "src/data.txt" 内容为 "foo\nbar\nbaz"
      当 调用grep 模式 "nonexistent" 路径 "src/data.txt"
      那么 结果应该为空或提示无匹配

    场景: grep上限
      假如 存在文件 "src/many.txt" 包含20行 "match"
      当 调用grep 模式 "match" 路径 "src/many.txt" 限制 5
      那么 恰好有 5 条匹配

    场景: grep忽略情形
      假如 存在文件 "src/case.txt" 内容为 "Hello World\nHELLO WORLD\nhello world"
      当 调用grep 不区分大小写 模式 "hello" 路径 "src/case.txt"
      那么 共有 3 条匹配

    场景: grep字面
      假如 存在文件 "src/literal.txt" 内容为 "function(x)\nfunction(y)\nfn.call()"
      当 调用grep 字面量模式 "fn.call()" 路径 "src/literal.txt"
      那么 结果包含 "fn.call()"
  @req:r1159
  规则: find fd
    Find 工具 须使用 fd，尊重 gitignore，返回 Posix 相对路径并支持 limit。

    场景: 查找简单
      假如 存在文件 "src/main.rs"
      并且 存在文件 "src/lib.rs"
      并且 存在文件 "src/main.txt"
      当 调用find 模式 "*.rs" 路径 "src"
      那么 结果包含 "main.rs"
      并且 结果包含 "lib.rs"
      并且 结果不包含 "main.txt"

    场景: 查找递归
      假如 存在文件 "src/lib.rs"
      并且 存在文件 "src/sub/mod.rs"
      并且 存在文件 "tests/test.rs"
      当 调用find 模式 "**/*.rs" 路径 "."
      那么 结果包含 "src/lib.rs"
      并且 结果包含 "src/sub/mod.rs"
      并且 结果包含 "tests/test.rs"

    场景: 查找上限
      假如 存在 50 个文件匹配模式
      当 调用find 模式 "*.log" 路径 "." 限制 10
      那么 恰好有 10 条结果

    场景: 查找无匹配
      当 调用find 模式 "*.nonexistent" 路径 "."
      那么 结果包含 "No files found" 或 "未找到文件"
  @req:r1160
  规则: bash 中止
    Bash 工具 须支持 CancellationToken 杀进程树并合并流式输出。

    场景: bash已合并流
      当 调用bash命令 "echo out && echo err >&2 && echo out2"
      那么 stdout 和 stderr 合并输出包含 "out"
      并且 stdout 和 stderr 合并输出包含 "err"
      并且 stdout 和 stderr 合并输出包含 "out2"

    场景: bash取消
      当 调用bash命令 "sleep 60"
      并且 在500ms后发送取消信号
      那么 命令应该失败 包含取消错误
  @req:r1136
  规则: read 截断
    Read 工具 须在 2000 行或 50KB 处截断，并对 offset 越界报告剩余行提示。

    场景: 读取偏移上限
      假如 存在文件 "src/multi.txt" 内容为 "第1行\n第2行\n第3行\n第4行\n第5行"
      当 调用read工具 路径 "src/multi.txt" 偏移 2 限制 2
      那么 内容为 "第2行\n第3行"
      并且 总行数为 5
      并且 偏移量为 2

    场景: 读取偏移越界
      假如 存在文件 "src/short.txt" 内容为 "只有一行\n"
      当 调用read工具 路径 "src/short.txt" 偏移 10
      那么 内容为空
      并且 偏移量为 10

    场景: 读取截断
      假如 存在文件 "src/large.txt" 包含10000行内容
      当 调用read工具 路径 "src/large.txt"
      那么 输出被截断
      并且 如果截断则显示剩余行提示
  @req:r1137
  规则: infra
    System 须提供 TruncationResult、XxxOperations trait、FileMutationQueue 与 CancellationToken。

    场景: 基建可用
      假如 内置工具集已构造
      当 分别执行 read / write / edit / bash / grep / find / ls
      那么 各工具返回成功结果且无 panic
  @req:r1138
  规则: ToolSet 单元操作
    工具容器 ToolSet 须仅提供构建期单元操作：empty、from_iter、plus、remove、merge、retain（带谓词）；不得提供运行时 allow/exclude 过滤 API 或 wrap_with_hooks 方法。按名 get 与遍历最终集合仍可用。

    场景: 工具集单测操作
      假如 工具集含 read 与 grep
      当 plus(bash) 然后 remove(grep)
      那么 最终工具集含 read 与 bash 且不含 grep
  @req:r1139
  规则: OutputAccumulator
    工具执行 须支持 OutputAccumulator：增量缓冲接收流式数据、在内存保留可配置尾部窗口、总量超滚动阈值时透明溢出到临时文件，并产出含截断信息的最终快照。

    场景: 小输出
      假如 累加器接收 100 字节
      当 调用 finish
      那么 snapshot.content 含 100 字节，未创建临时文件

    场景: 溢出临时文件
      假如 累加器接收 2x max_bytes
      当 调用 finish
      那么 快照内容为截断尾部，full_output_path 指向含完整输出的临时文件
  @req:r1140
  规则: bash 累加器
    Bash 工具 须对 stdout/stderr 流式输出使用 OutputAccumulator，替代 String 拼接；配置 max_lines 100 与 max_bytes DEFAULT_MAX_BYTES。

    场景: 场景bashstderr
      当 调用bash命令 "echo 错误信息 >&2"
      那么 stdout 和 stderr 合并输出包含 "错误信息"

    场景: bash使用累加器
      假如 bash 工具即将执行 echo hello
      当 执行 bash echo hello
      那么 返回 output:hello 且未触发临时文件落盘
  @req:r1147
  规则: bash-tool-result-no-full-dump
    Bash 工具返回的 JSON（流式与非流式）在截断时 须仅含截断视图（stdout/combined 为 display_content，含 Full output 脚注）与 truncated/full_output_path；不得再附带未截断的完整 stdout 或 stderr 载荷。未截断时 stdout/combined 须为完整合并输出且无 Full output 脚注。

    场景: bash结果无完整转储
      当 调用bash命令 "yes yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy | head -n 3000"
      那么 结果含 Full output 脚注
      并且 bash 结果 JSON 无未截断全量 stdout 字段载荷
  @req:r1131
  规则: 模糊匹配
    System 须对 edit oldText 做模糊匹配：NFKC 规范化、智能引号转 ASCII、破折号转连字符、剥离尾部空白，再逐步放宽匹配。

    场景: 编辑Unicode
      假如 存在文件 "src/quotes.rs" 内容为 "let msg = \"hello world\";"
      当 调用edit工具 路径 "src/quotes.rs" 将 "let msg = \"hello world\";" 替换为 "let msg = \"hi earth\";"
      那么 文件 "src/quotes.rs" 应该包含 "hi earth"

    场景: 模糊
      假如 文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价
      当 应用 edit
      那么 模糊匹配成功并写入替换
  @req:r1132
  规则: 行尾风格
    System 须在编辑前检测文件行尾（CRLF vs LF），应用变更后恢复原有风格。

    场景: 编辑CRLF
      假如 存在文件 "src/windows.rs" 使用CRLF行尾 内容为 "// Windows 风格\n// 第二行"
      当 调用edit工具 路径 "src/windows.rs" 将 "Windows 风格" 替换为 "Unix 风格"
      那么 文件 "src/windows.rs" 应该包含 "Unix 风格"
      并且 文件 "src/windows.rs" 应该包含 "第二行"
  @req:r1133
  规则: 跨行 span 匹配
    精确匹配失败时，System 须支持跨不同行段匹配多行 oldText span。

    场景: 跨度
      假如 oldText 跨 5 行但中间一行精确匹配失败
      当 以 span 匹配应用 edit
      那么 滑动窗口匹配成功
  @req:r1134
  规则: diff 输出
    编辑操作后，System 须使用 similar crate 计算结构化 diff 供审阅。
    场景: 结构化diff应用带偏移块
      假如 临时目录存在含行偏移的 unified diff 文件
      当 经补丁应用执行 edit
      那么 模糊匹配补丁应用成功

  @req:r1141
  规则: 工具定义包含 schema
    展示类型 ToolDefinition 须包含 XyToolSchema 字段，而非重复 name/description/parameters 字段。
    场景: 模式驱动必需参数
      假如 工具需要必填字符串参数 file_path
      当 以空参调用该工具
      那么 调用失败且返回 MissingArgument 错误码

  @req:r1142
  规则: ToolSet 在 agent 终态
    ToolSet 须作为构建期终态编排状态留在 agent 层：聚合具体工具并组合单元操作；构建后交给一轮的集合即终态，无运行时过滤。具体工具由组合根注入。
    场景: 工具集为构建终态
      假如 工具集含 read 与 grep
      当 plus(bash) 然后 remove(grep)
      那么 最终工具集含 read 与 bash 且不含 grep

  @req:r1143
  规则: 工具相关类型统一前缀
    ToolExecutionMode 须重命名为 XyToolExecutionMode；protocol 工具边界及其签名类型 须一致使用 Xy 前缀命名。
    场景: 执行模式携带Xy前缀
      当 读取工具执行类命名
      那么 执行类类型为 XyToolExecutionMode

  @req:r36
  规则: MCP 工具经 XyTool 接入
    MCP 工具 须仅通过 XyTool 端口进入 agent 工具集；rmcp 具体类型 不得泄漏到 agent/ 或 domain/。
    场景: MCP工具回车作为XyTool
      当 以 fixture MCP 配置装配 ping 工具
      那么 装配出 mcp 前缀的 XyTool 且连接在册

  @req:r1145
  规则: read-image-parts
    Read 工具在路径为支持的图片文件时 须经 resize/multimodal 准备产出含 AgentPart::Image（base64 data + media_type）的 tool result parts，并附简短 text note；不得仅返回「[Image file: …]」占位字符串作为唯一结果。文本文件行为保持既有截断语义。工具端口 须允许 execute 结果以 parts 形式进入 history（默认实现可将纯文本包成 Text part）。

    场景: PNG产出图片部件
      假如 临时目录有小 PNG
      当 调用 read
      那么 tool result parts 含 Image 且 data 非空

    场景: 文本仍可用
      假如 临时目录有 .txt
      当 调用 read
      那么 返回文本内容且无 Image part
  @req:r1146
  规则: write-edit-llm-result-quiet
    write/edit 成功时，进入会话 history 并经 project_for_llm 发往模型的 tool result 正文 须为短成功句（对齐 pi）；不得把完整 content 回灌或嵌入 unified/display_diff。失败时 须保留可读错误原因。UI 仍 须能取得 display_diff（或等价）以渲染编辑块。

    场景: 写入字节计数
      当 调用write工具 路径 "src/size.txt" 内容 "hello"
      那么 结果包含 "success"
  @req:r1150
  规则: builtin-tool-concurrency-class
    内置工具 须声明并发类：read/grep/find/ls 为 ParallelSafe；write/edit/bash 与 todo_rewrite/todo_update 为 Barrier（或等价 Sequential 映射）。调度层在 barrier_parallel 下 须尊重该类；FileMutationQueue 可并存作同 path 纵深防御。由单测覆盖 default_tools 分类表，不得为静态表单独扩 BDD step。
    场景: 内置宣告并发类
      假如 未配置工具批模式且 mock 模型同 turn 发出两个可并行假工具
      当 运行 AgentRuntime
      那么 两工具执行时间重叠

  @req:r1151
  规则: dynamic-tool-mcp-hard-barrier
    公开名以 `mcp__` 为前缀（过渡期可识别遗留 `mcp-` / `mcp_` / `mcp:`）的工具（含 McpToolAdapter 与任何同前缀注册）在批调度分类中 须为 Barrier，无论其 trait 并发声明或配置如何；不得提供 glob、配置名名单或 MCP annotation 将其升为 ParallelSafe。非 mcp 自注册工具缺省 须为 Barrier，MAY 经 ToolConcurrency 显式标 ParallelSafe。由单测覆盖，不得为静态默认单独扩 BDD step。
    场景: MCP带前缀为硬屏障
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、mcp 假工具、ParallelSafe
      当 运行 AgentRuntime
      那么 mcp 假工具与两侧 ParallelSafe 均无执行时间重叠

  @req:r1152
  规则: ask-tui-only-builtin
    内置工具 ask 须以 TypedTool 存在于 infra；schema 须含 questions 数组（每题 id/prompt/mode/options，option.description 可选）；execute 成功结果 须为 status 为 skipped 或 answered 的 JSON。default_tools 不得包含 ask；仅 TUI 装配路径 MAY 经 ToolSet plus 注入。ask 须为 Barrier Sequential。Abort 不得伪装为 skipped。
    场景: 问卷内置于TUI工具集
      假如 产品以 TUI 面启动
      当 查询装配后的 ToolSet
      那么 含名为 ask 的内置工具

  @req:r1161
  规则: 工具执行会话工作区
    模型侧内置工具 须以运行时注入的会话工作区为基：bash 子进程在该目录内启动；文件类工具（read/write/edit/ls/grep/find）的相对路径相对该目录解析；绝对路径行为不变。运行时未注入工作区时 须回落 Host 进程 cwd。同一回合（含工具环）内工作区 须保持开跑时冻结值，不得随进程外状态漂移。

    场景: ls缺省工作目录
      假如 工作区根目录存在文件 "in_root.txt"
      当 调用ls 不传路径参数
      那么 结果列出 "in_root.txt"

    场景: 工具写入于会话工作区
      假如 有一个临时工作目录
      当 以会话工作区调用write工具 路径 "c2335-probe/notes.txt" 内容 "落点正确"
      那么 文件 "c2335-probe/notes.txt" 在会话工作区内存在
      并且 文件 "c2335-probe/notes.txt" 不在进程工作目录

    场景: bash运行于会话工作区
      假如 有一个临时工作目录
      当 以会话工作区调用bash命令 "pwd"
      那么 bash 输出为会话工作区目录
