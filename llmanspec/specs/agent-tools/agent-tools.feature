# language: zh-CN
# capability: agent-tools
# purpose: 工具系统 — 工具定义、执行分发、文件类内置与 Todo 三工具、路径工具与并发类。
# scope: agent 层工具, infra 层工具实现, protocol 层

功能: agent-tools
  背景:
    假如 有一个临时工作目录
    并且 存在文件 "src/main.rs" 内容为 "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}"

  @req:r32 @human
  场景: 工具 trait 前缀
    - System MUST 将主工具接口定义为 XyTool；trait 名已带 Xy 前缀，MUST 与 protocol 边界其余命名一致。

  @req:r42 @human
  场景: 默认内置工具闭集
    - 产品 default_tools（Print 与 TUI builtins 基座）MUST 包含且以闭集提供：read、bash、edit、write、grep、find、ls、todo_list、todo_rewrite、todo_update（共 10 个）。ask 等面专属工具 MUST NOT 计入该闭集（见 t28）。Todo 工具语义见 agent-todo。

  @req:r48 @human
  场景: 补丁应用
    - System MUST 使用 fudiff 模糊匹配应用 AI 生成的补丁，并以 patch crate 精确匹配作为回退。

  @req:r4 @human
  场景: 读文件大小限制
    - Read 工具 MUST 拒绝超过 MAX_FILE_SIZE（默认 10MB）的文件，并返回清晰错误信息。

  @req:r5 @human
  场景: bash 流式输出
    - Bash 工具 MUST 流式输出 stdout/stderr 并设字节上限；超限时 MUST 终止子进程。

  @req:r6 @human
  场景: 整数转换校验
    - 工具 MUST 在转换前校验数值参数（timeout、max_results）处于正数安全范围内；timeout 若出现则 MUST 为正整数秒，超过全局上界的值 MUST 被程序钳制到上界而非拒绝，MUST NOT 将 0 解释为无限。

  @req:t24 @human
  场景: 工具超时默认有界
    - 命令类工具（bash、grep、find）的执行等待由程序最高权限管理：timeout 为模型侧请求——正整数秒被接受并钳制到全局上界（600 秒），省略时按工具默认（bash 120 秒、grep/find 60 秒）；计时器 MUST 必然武装，到期终止并返回 timeout 类错误；MUST NOT 存在无限语义或可绕过计时的输入。任何外部等待 MUST NOT 超过全局兜底上界（600 秒）。流式与非流式路径共用同一语义。

  @req:t25 @human
  场景: grep-find-timeout-schema
    - grep 与 find 的 parameters schema MUST 暴露可选 timeout（秒）；行为遵循 t24。

  @req:t29 @human
  场景: fs 工具有界
    - read、write、edit、ls 四个文件类工具的执行等待 MUST 有默认上限（30 秒），到期以 timeout 类错误呈现；MUST NOT 向模型暴露 per-call timeout 参数。

  @req:r7 @human
  场景: find 禁止绝对路径逃逸
    - Find 工具 MUST 拒绝绝对 pattern，或 MUST 规范化结果以验证其仍在 root 目录内。

  @req:r8 @human
  场景: 安全 UTF-8 截断
    - 输出截断 MUST 使用字符边界感知切片，防止 UTF-8 panic。

  @req:r9 @human
  场景: 工具参数辅助函数
    - 工具系统 MUST 提供共享参数提取辅助函数，消除各工具实现中重复的 JSON 字段解析。

  @req:r10 @human
  场景: 禁止全局 dead_code 允许
    - crate 根 MUST NOT 使用全局 #![allow(dead_code)]；dead code 抑制 MUST 限定于单项并附理由。

  @req:r11 @human
  场景: 工具错误类型
    - System MUST 定义 XyToolError，含结构化错误类别（invalid_args / execution_failed / permission_denied / timeout），独立于 adk-core。

  @req:t1 @human
  场景: traits
    - System MUST 定义 XyTool trait，execute 接受 CancellationToken。

  @req:t2 @human
  场景: 默认内置工具闭集·t2
    - System MUST 实现与 r42 相同的 10 个 default_tools 闭集成员：read、bash、edit、write、grep、find、ls、todo_list、todo_rewrite、todo_update。文件类七工具行为对齐既有 pi 等价语义；Todo 三工具行为见 agent-todo。

  @req:t3 @human
  场景: edit 多段编辑
    - Edit 工具 MUST 接受 path 与 edits 数组（oldText、newText），对照原始内容匹配。

  @req:t4 @human
  场景: edit 校验
    - Edit 工具 MUST 拒绝重叠编辑、非唯一 oldText、空 oldText 与无变化编辑。

  @req:t5 @human
  场景: edit Unicode
    - Edit 工具 MUST 将 CRLF 规范为 LF、剥离 UTF-8 BOM，并通过 NFKC 规范化做模糊匹配。

  @req:t6 @human
  场景: edit diff
    - Edit 工具 MUST 对编辑前后的整文件内容产生 unified patch 与带行号的展示 diff（display_diff MUST 为 context hunk，MUST NOT 把全文件 Equal 行展开进展示；展示行数 MUST 有硬上限并在超限时标注截断）；发往模型的 tool result 正文 MUST NOT 嵌入完整 diff/display_diff。

  @req:t7 @human
  场景: grep ripgrep
    - Grep 工具 MUST 使用 ripgrep，支持 regex、glob、ignoreCase、literal、context、limit。

  @req:t8 @human
  场景: find fd
    - Find 工具 MUST 使用 fd，尊重 gitignore，返回 Posix 相对路径并支持 limit。

  @req:t9 @human
  场景: bash 中止
    - Bash 工具 MUST 支持 CancellationToken 杀进程树并合并流式输出。

  @req:t10 @human
  场景: read 截断
    - Read 工具 MUST 在 2000 行或 50KB 处截断，并对 offset 越界报告剩余行提示。

  @req:t11 @human
  场景: infra
    - System MUST 提供 TruncationResult、XxxOperations trait、FileMutationQueue 与 CancellationToken。

  @req:t12 @human
  场景: ToolSet 单元操作
    - 工具容器 ToolSet MUST 仅提供构建期单元操作：empty、from_iter、plus、remove、merge、retain（带谓词）；MUST NOT 提供运行时 allow/exclude 过滤 API 或 wrap_with_hooks 方法。按名 get 与遍历最终集合仍可用。

  @req:t14 @human
  场景: OutputAccumulator
    - 工具执行 MUST 支持 OutputAccumulator：增量缓冲接收流式数据、在内存保留可配置尾部窗口、总量超滚动阈值时透明溢出到临时文件，并产出含截断信息的最终快照。

  @req:t15 @human
  场景: bash 累加器
    - Bash 工具 MUST 对 stdout/stderr 流式输出使用 OutputAccumulator，替代 String 拼接；配置 max_lines 100 与 max_bytes DEFAULT_MAX_BYTES。

  @req:t23 @human
  场景: bash-tool-result-no-full-dump
    - Bash 工具返回的 JSON（流式与非流式）在截断时 MUST 仅含截断视图（stdout/combined 为 display_content，含 Full output 脚注）与 truncated/full_output_path；MUST NOT 再附带未截断的完整 stdout 或 stderr 载荷。未截断时 stdout/combined MUST 为完整合并输出且无 Full output 脚注。

  @req:e1 @human
  场景: 模糊匹配
    - System MUST 对 edit oldText 做模糊匹配：NFKC 规范化、智能引号转 ASCII、破折号转连字符、剥离尾部空白，再逐步放宽匹配。

  @req:e2 @human
  场景: 行尾风格
    - System MUST 在编辑前检测文件行尾（CRLF vs LF），应用变更后恢复原有风格。

  @req:e3 @human
  场景: 跨行 span 匹配
    - 精确匹配失败时，System MUST 支持跨不同行段匹配多行 oldText span。

  @req:e4 @human
  场景: diff 输出
    - 编辑操作后，System MUST 使用 similar crate 计算结构化 diff 供审阅。

  @req:t17 @human
  场景: 工具定义包含 schema
    - 展示类型 ToolDefinition MUST 包含 XyToolSchema 字段，而非重复 name/description/parameters 字段。

  @req:t18 @human
  场景: ToolSet 在 agent 终态
    - ToolSet MUST 作为构建期终态编排状态留在 agent 层：聚合具体工具并组合单元操作；构建后交给一轮的集合即终态，无运行时过滤。具体工具由组合根注入。

  @req:t19 @human
  场景: 工具相关类型统一前缀
    - ToolExecutionMode MUST 重命名为 XyToolExecutionMode；protocol 工具边界及其签名类型 MUST 一致使用 Xy 前缀命名。

  @req:r36 @human
  场景: MCP 工具经 XyTool 接入
    - MCP 工具 MUST 仅通过 XyTool 端口进入 agent 工具集；rmcp 具体类型 MUST NOT 泄漏到 agent/ 或 domain/。

  @req:t21 @human
  场景: read-image-parts
    - Read 工具在路径为支持的图片文件时 MUST 经 resize/multimodal 准备产出含 AgentPart::Image（base64 data + media_type）的 tool result parts，并附简短 text note；MUST NOT 仅返回「[Image file: …]」占位字符串作为唯一结果。文本文件行为保持既有截断语义。工具端口 MUST 允许 execute 结果以 parts 形式进入 history（默认实现可将纯文本包成 Text part）。

  @req:t22 @human
  场景: write-edit-llm-result-quiet
    - write/edit 成功时，进入会话 history 并经 project_for_llm 发往模型的 tool result 正文 MUST 为短成功句（对齐 pi）；MUST NOT 把完整 content 回灌或嵌入 unified/display_diff。失败时 MUST 保留可读错误原因。UI 仍 MUST 能取得 display_diff（或等价）以渲染编辑块。

  @req:t26 @human
  场景: builtin-tool-concurrency-class
    - 内置工具 MUST 声明并发类：read/grep/find/ls 为 ParallelSafe；write/edit/bash 与 todo_list/todo_rewrite/todo_update 为 Barrier（或等价 Sequential 映射）。调度层在 barrier_parallel 下 MUST 尊重该类；FileMutationQueue 可并存作同 path 纵深防御。由单测覆盖 default_tools 分类表，MUST NOT 为静态表单独扩 BDD step。

  @req:t27 @human
  场景: dynamic-tool-mcp-hard-barrier
    - 公开名以 `mcp__` 为前缀（过渡期可识别遗留 `mcp-` / `mcp_` / `mcp:`）的工具（含 McpToolAdapter 与任何同前缀注册）在批调度分类中 MUST 为 Barrier，无论其 trait 并发声明或配置如何；MUST NOT 提供 glob、配置名名单或 MCP annotation 将其升为 ParallelSafe。非 mcp 自注册工具缺省 MUST 为 Barrier，MAY 经 ToolConcurrency 显式标 ParallelSafe。由单测覆盖，MUST NOT 为静态默认单独扩 BDD step。

  @req:t28 @human
  场景: ask-tui-only-builtin
    - 内置工具 ask MUST 以 TypedTool 存在于 infra；schema MUST 含 questions 数组（每题 id/prompt/mode/options，option.description 可选）；execute 成功结果 MUST 为 status 为 skipped 或 answered 的 JSON。default_tools MUST NOT 包含 ask；仅 TUI 装配路径 MAY 经 ToolSet plus 注入。ask MUST 为 Barrier Sequential。Abort MUST NOT 伪装为 skipped。

  @req:ws1 @human
  场景: 工具执行会话工作区
    - 模型侧内置工具 MUST 以运行时注入的会话工作区为基：bash 子进程在该目录内启动；文件类工具（read/write/edit/ls/grep/find）的相对路径相对该目录解析；绝对路径行为不变。运行时未注入工作区时 MUST 回落 Host 进程 cwd。同一回合（含工具环）内工作区 MUST 保持开跑时冻结值，MUST NOT 随进程外状态漂移。
  @executable @req:t2
  场景: read-entire
    假如 存在文件 "src/hello.rs" 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
    当 调用read工具 路径 "src/hello.rs"
    那么 内容为 "fn main() {\n    println!(\"Hello, world!\");\n}"
    并且 总行数为 3

  @executable @req:t10
  场景: read-offset-limit
    假如 存在文件 "src/multi.txt" 内容为 "第1行\n第2行\n第3行\n第4行\n第5行"
    当 调用read工具 路径 "src/multi.txt" 偏移 2 限制 2
    那么 内容为 "第2行\n第3行"
    并且 总行数为 5
    并且 偏移量为 2

  @executable @req:r11
  场景: read-missing
    当 调用read工具 路径 "nonexistent.txt"
    那么 调用失败 包含错误信息 "not found" 或 "No such file" 或 "不存在"

  @executable @req:t10
  场景: read-offset-oob
    假如 存在文件 "src/short.txt" 内容为 "只有一行\n"
    当 调用read工具 路径 "src/short.txt" 偏移 10
    那么 内容为空
    并且 偏移量为 10

  @executable @req:t10
  场景: read-truncate
    假如 存在文件 "src/large.txt" 包含10000行内容
    当 调用read工具 路径 "src/large.txt"
    那么 输出被截断
    并且 如果截断则显示剩余行提示

  @executable @req:r11
  场景: read-missing-path
    当 调用read 不传路径参数
    那么 调用失败 包含验证错误

  @executable @req:t2
  场景: write-new
    当 调用write工具 路径 "src/output.txt" 内容 "hello world"
    那么 文件 "src/output.txt" 应该存在
    并且 文件 "src/output.txt" 内容为 "hello world"

  @executable @req:t2
  场景: write-parents
    当 调用write工具 路径 "nested/deep/dir/file.txt" 内容 "深层内容"
    那么 文件 "nested/deep/dir/file.txt" 应该存在
    并且 文件 "nested/deep/dir/file.txt" 内容为 "深层内容"

  @executable @req:t2
  场景: write-overwrite
    假如 存在文件 "src/existing.txt" 内容为 "旧内容"
    当 调用write工具 路径 "src/existing.txt" 内容 "新内容"
    那么 文件 "src/existing.txt" 内容为 "新内容"

  @executable @req:t22
  场景: write-byte-count
    当 调用write工具 路径 "src/size.txt" 内容 "hello"
    那么 结果包含 "success"

  @executable @req:r11
  场景: write-missing-path
    当 调用write 不传路径参数
    那么 调用失败 包含验证错误

  @executable @req:r11
  场景: write-missing-content
    当 调用write工具 路径 "test.txt" 不传内容
    那么 调用失败 包含验证错误

  @executable @req:t2
  场景: bash-echo
    当 调用bash命令 "echo hello world"
    那么 退出码为 0
    并且 stdout 包含 "hello world"

  @executable @req:t15
  场景: bash-stderr
    当 调用bash命令 "echo 错误信息 >&2"
    那么 stdout 和 stderr 合并输出包含 "错误信息"

  @executable @req:t2
  场景: bash-exit-code
    当 调用bash命令 "exit 42"
    那么 退出码为 42

  @executable @req:t24
  场景: bash-timeout
    当 调用bash命令 "sleep 10" 超时 1 秒
    那么 命令应该失败 包含超时错误

  @executable @req:t24
  场景: bash-omit-timeout-completes
    当 调用bash命令 "sleep 2"
    那么 退出码为 0

  @executable @req:t9
  场景: bash-merged-streams
    当 调用bash命令 "echo out && echo err >&2 && echo out2"
    那么 stdout 和 stderr 合并输出包含 "out"
    并且 stdout 和 stderr 合并输出包含 "err"
    并且 stdout 和 stderr 合并输出包含 "out2"

  @executable @req:r5
  场景: bash-truncate
    当 调用bash命令 "yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx | head -n 3000"
    那么 输出被截断
    并且 截断详情显示达到字节或行限制
    并且 结果含 Full output 脚注
    并且 bash 结果 JSON 无未截断全量 stdout 字段载荷

  @executable @req:t23
  场景: bash-result-no-full-dump
    当 调用bash命令 "yes yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy | head -n 3000"
    那么 结果含 Full output 脚注
    并且 bash 结果 JSON 无未截断全量 stdout 字段载荷

  @executable @req:t9
  场景: bash-cancel
    当 调用bash命令 "sleep 60"
    并且 在500ms后发送取消信号
    那么 命令应该失败 包含取消错误

  @executable @req:r11
  场景: bash-missing-cmd
    当 调用bash 不传命令参数
    那么 调用失败 包含验证错误

  @executable @req:t3
  场景: edit-single
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"你好\");"
    那么 文件 "src/main.rs" 应该包含 "你好"

  @executable @req:t3
  场景: edit-multi
    假如 存在文件 "src/lib.rs" 内容为 "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }"
    当 调用edit工具 路径 "src/lib.rs" 将 "a + b" 替换为 "a.wrapping_add(b)"
    并且 调用edit工具 路径 "src/lib.rs" 将 "a - b" 替换为 "a.wrapping_sub(b)"
    那么 文件 "src/lib.rs" 应该包含 "a.wrapping_add(b)"
    并且 文件 "src/lib.rs" 应该包含 "a.wrapping_sub(b)"

  @executable @req:t4
  场景: edit-overlap
    假如 存在文件 "src/overlap.rs" 内容为 "fn hello_world() {\n    println!(\"hello world\");\n}"
    当 调用edit工具 路径 "src/overlap.rs" 做重叠替换
    那么 edit调用应该失败 包含错误信息 "Overlapping edits"

  @executable @req:t4
  场景: edit-nonunique
    假如 存在文件 "src/dup.rs" 内容为 "let x = 1;\nlet x = 2;"
    当 调用edit工具 路径 "src/dup.rs" 做重复替换
    那么 edit调用应该失败 包含错误信息 "not unique"

  @executable @req:t4
  场景: edit-empty-old
    当 调用edit工具 路径 "src/main.rs" 将 "" 替换为 "foo"
    那么 edit调用应该失败 包含错误信息 "empty" 或 "空"

  @executable @req:t4
  场景: edit-noop
    当 调用edit工具 路径 "src/main.rs" 将 "println!(\"hello\");" 替换为 "println!(\"hello\");"
    那么 edit调用应该失败 包含错误信息 "identical" 或 "No changes" 或 "no change" 或 "not unique"

  @executable @req:e2
  场景: edit-crlf
    假如 存在文件 "src/windows.rs" 使用CRLF行尾 内容为 "// Windows 风格\n// 第二行"
    当 调用edit工具 路径 "src/windows.rs" 将 "Windows 风格" 替换为 "Unix 风格"
    那么 文件 "src/windows.rs" 应该包含 "Unix 风格"
    并且 文件 "src/windows.rs" 应该包含 "第二行"

  @executable @req:t5
  场景: edit-bom
    假如 存在文件 "src/bom.txt" 带UTF8_BOM 内容为 "Hello World"
    当 调用edit工具 路径 "src/bom.txt" 将 "Hello World" 替换为 "Hello BOM"
    那么 文件 "src/bom.txt" 应该保留UTF8_BOM
    并且 文件 "src/bom.txt" 应该包含 "Hello BOM"

  @executable @req:e1
  场景: edit-unicode
    假如 存在文件 "src/quotes.rs" 内容为 "let msg = \"hello world\";"
    当 调用edit工具 路径 "src/quotes.rs" 将 "let msg = \"hello world\";" 替换为 "let msg = \"hi earth\";"
    那么 文件 "src/quotes.rs" 应该包含 "hi earth"

  @executable @req:t6 @req:e4
  场景: edit-diff
    假如 存在文件 "src/diff_test.rs" 内容为 "第1行\n第2行\n第3行"
    当 调用edit工具 路径 "src/diff_test.rs" 将 "第2行" 替换为 "第二行"
    那么 结果包含 "@@"

  @executable @req:t7
  场景: grep-basic
    假如 存在文件 "src/data.txt" 内容为 "apple\nbanana\ncherry\napple pie\norange"
    当 调用grep 模式 "apple" 路径 "src/data.txt"
    那么 匹配结果包含第1行的 "apple"
    并且 匹配结果包含第4行的 "apple pie"
    并且 共有 2 条匹配

  @executable @req:t7
  场景: grep-no-match
    假如 存在文件 "src/data.txt" 内容为 "foo\nbar\nbaz"
    当 调用grep 模式 "nonexistent" 路径 "src/data.txt"
    那么 结果应该为空或提示无匹配

  @executable @req:t7
  场景: grep-limit
    假如 存在文件 "src/many.txt" 包含20行 "match"
    当 调用grep 模式 "match" 路径 "src/many.txt" 限制 5
    那么 恰好有 5 条匹配

  @executable @req:t7
  场景: grep-ignore-case
    假如 存在文件 "src/case.txt" 内容为 "Hello World\nHELLO WORLD\nhello world"
    当 调用grep 不区分大小写 模式 "hello" 路径 "src/case.txt"
    那么 共有 3 条匹配

  @executable @req:t7
  场景: grep-literal
    假如 存在文件 "src/literal.txt" 内容为 "function(x)\nfunction(y)\nfn.call()"
    当 调用grep 字面量模式 "fn.call()" 路径 "src/literal.txt"
    那么 结果包含 "fn.call()"

  @executable @req:r11
  场景: grep-missing-pattern
    当 调用grep 不传模式参数
    那么 调用失败 包含验证错误

  @executable @req:t8
  场景: find-simple
    假如 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在文件 "src/main.txt"
    当 调用find 模式 "*.rs" 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果不包含 "main.txt"

  @executable @req:t8
  场景: find-recursive
    假如 存在文件 "src/lib.rs"
    并且 存在文件 "src/sub/mod.rs"
    并且 存在文件 "tests/test.rs"
    当 调用find 模式 "**/*.rs" 路径 "."
    那么 结果包含 "src/lib.rs"
    并且 结果包含 "src/sub/mod.rs"
    并且 结果包含 "tests/test.rs"

  @executable @req:t8
  场景: find-limit
    假如 存在 50 个文件匹配模式
    当 调用find 模式 "*.log" 路径 "." 限制 10
    那么 恰好有 10 条结果

  @executable @req:t8
  场景: find-no-match
    当 调用find 模式 "*.nonexistent" 路径 "."
    那么 结果包含 "No files found" 或 "未找到文件"

  @executable @req:r11
  场景: find-bad-path
    当 调用find 模式 "*.rs" 路径 "/nonexistent/path"
    那么 调用失败 包含错误信息

  @executable @req:r7
  场景: find-absolute
    当 调用find工具 查找绝对路径失败
    那么 调用失败 包含错误信息

  @executable @req:t2
  场景: ls-empty
    假如 存在空目录 "empty_dir"
    当 调用ls工具 路径 "empty_dir"
    那么 结果指示目录为空

  @executable @req:t2
  场景: ls-entries
    假如 存在文件 "src/main.rs"
    并且 存在文件 "src/lib.rs"
    并且 存在目录 "src/subdir"
    当 调用ls工具 路径 "src"
    那么 结果包含 "main.rs"
    并且 结果包含 "lib.rs"
    并且 结果包含 "subdir"

  @executable @req:t2
  场景: ls-sorted
    假如 目录 "sorted" 中存在文件 "z.txt" "a.txt" "m.txt"
    当 调用ls工具 路径 "sorted"
    那么 条目按字母顺序排列

  @executable @req:ws1
  场景: ls-default-cwd
    假如 工作区根目录存在文件 "in_root.txt"
    当 调用ls 不传路径参数
    那么 结果列出 "in_root.txt"

  @executable @req:t2
  场景: ls-limit
    假如 目录 "many" 中存在 100 个文件
    当 调用ls工具 路径 "many" 限制 10
    那么 恰好有 10 条结果
    并且 结果指示达到条目限制

  @executable @req:r11
  场景: ls-missing
    当 调用ls工具 路径 "no_such_dir"
    那么 调用失败 包含错误信息

  @executable @req:r11
  场景: ls-not-dir
    假如 存在文件 "src/main.rs"
    当 调用ls工具 路径 "src/main.rs"
    那么 调用失败 包含错误信息

  @executable @req:r42
  场景: all-ten-tools-smoke
    假如 工具注册表含全部 10 个工具
    当 各工具以合法参数调用
    那么 各返回成功 ToolResult

  @executable @req:r48
  场景: fudiff-line-offset
    假如 临时目录存在含行偏移的 unified diff 文件
    当 经补丁应用执行 edit
    那么 模糊匹配补丁应用成功

  @executable @req:r4
  场景: large-file
    假如 存在 50MB 文件
    当 read 工具在无 offset/limit 时调用
    那么 工具返回文件过大错误

  @executable @req:r5
  场景: output-overflow
    假如 bash 运行 yes 命令
    当 stdout 超过 1MB 上限
    那么 子进程被杀并返回截断输出

  @executable @req:r6
  场景: negative-timeout
    假如 LLM 传入 timeout=-1
    当 bash 工具校验参数
    那么 工具以无效 timeout 错误拒绝

  @executable @req:r6
  场景: zero-timeout-rejected
    假如 LLM 传入 timeout=0
    当 bash 工具校验参数
    那么 工具以无效 timeout 错误拒绝

  @executable @req:r7
  场景: absolute-pattern
    假如 find 以 pattern='/etc/**' 调用
    当 安全已启用
    那么 工具返回错误或过滤结果至 root

  @executable @req:r8
  场景: multibyte-truncation
    假如 bash 输出在上限边界以不完整 UTF-8 序列结束
    当 调用 truncate_output
    那么 输出在字符边界安全截断且不 panic

  @executable @req:r9
  场景: require-str-missing-arg
    假如 工具需要必填字符串参数 file_path
    当 以空参调用该工具
    那么 调用失败且返回 MissingArgument 错误码

  @executable @req:r11
  场景: error-mapping
    假如 工具以 InvalidArgs 错误执行失败
    当 agent 循环收集工具结果
    那么 产生的 AgentEvent 含 error 类别

  @executable @req:t1
  场景: cancel
    假如 bash 工具正在 sleep 60
    当 发送取消信号
    那么 execute 返回 Cancelled 错误

  @executable @req:t2
  场景: registry
    假如 工具集含全部内置工具
    当 列举工具名
    那么 返回 10 个工具名

  @executable @req:t11
  场景: infra-works
    假如 内置工具集已构造
    当 分别执行 read / write / edit / bash / grep / find / ls
    那么 各工具返回成功结果且无 panic

  @executable @req:t12
  场景: toolset-unit-ops
    假如 工具集含 read 与 grep
    当 plus(bash) 然后 remove(grep)
    那么 最终工具集含 read 与 bash 且不含 grep

  @executable @req:t14
  场景: small-output
    假如 累加器接收 100 字节
    当 调用 finish
    那么 snapshot.content 含 100 字节，未创建临时文件

  @executable @req:t14
  场景: overflow-temp-file
    假如 累加器接收 2x max_bytes
    当 调用 finish
    那么 快照内容为截断尾部，full_output_path 指向含完整输出的临时文件

  @executable @req:t15
  场景: bash-uses-accumulator
    假如 bash 工具即将执行 echo hello
    当 执行 bash echo hello
    那么 返回 output:hello 且未触发临时文件落盘

  @executable @req:e1
  场景: fuzzy
    假如 文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价
    当 应用 edit
    那么 模糊匹配成功并写入替换

  @executable @req:e3
  场景: span
    假如 oldText 跨 5 行但中间一行精确匹配失败
    当 以 span 匹配应用 edit
    那么 滑动窗口匹配成功

  @executable @req:t21
  场景: png-yields-image-part
    假如 临时目录有小 PNG
    当 调用 read
    那么 tool result parts 含 Image 且 data 非空

  @executable @req:t21
  场景: text-still-works
    假如 临时目录有 .txt
    当 调用 read
    那么 返回文本内容且无 Image part

  @executable @req:ws1
  场景: tool-write-in-session-workspace
    假如 有一个临时工作目录
    当 以会话工作区调用write工具 路径 "c2335-probe/notes.txt" 内容 "落点正确"
    那么 文件 "c2335-probe/notes.txt" 在会话工作区内存在
    并且 文件 "c2335-probe/notes.txt" 不在进程工作目录

  @executable @req:ws1
  场景: bash-runs-in-session-workspace
    假如 有一个临时工作目录
    当 以会话工作区调用bash命令 "pwd"
    那么 bash 输出为会话工作区目录
