# language: zh-CN
# capability: infra-bash
# purpose: 经 shell 子进程执行 Bash 命令，含流式输出与取消。
# scope: src/infra/bash_exec/, src/protocol/

功能: infra-bash

  @req:be1 @human
  场景: 流式执行器
    - System MUST 提供 XyBashExecutor，经 BashExecOpts 执行 shell 命令，携带可选 CancellationToken 与可选有界 mpsc chunk_tx 输出字节；chunk_tx 为 Some 时执行器 MUST 在该通道发送输出块（Full 时发送方合并）；为 None 时 MUST 静默累积且不要求回调。原 on_chunk 回调签名 MUST NOT 保留为公共端口。

  @req:be2 @human
  场景: 中止与取消
    - BashExecutor MUST 支持取消，杀死整个进程组并将结果标为 cancelled。

  @req:be3 @human
  场景: 输出截断
    - BashExecutor MUST 将输出截断到配置 max bytes，超限时将完整输出溢出到临时文件；返回给调用方/会话的 output MUST 为截断尾部，并 MUST 追加 pi 形脚注 `[Full output: <path>. Truncated: <N> lines shown (<limit> limit)]`（path 不可用时用 `(unavailable)`）；未截断 MUST NOT 追加该脚注。

  @req:be4 @human
  场景: 会话条目
    - 新写入的 bang-bash MUST 持久化为 SessionEntry::Message（type=message），其 message 为 Env bashExecution（role=bashExecution），携带 command、output、exit_code、cancelled、truncated、full_output_path、exclude_from_context。MUST NOT 再对新写入使用顶层 type=bashExecution 变体。读路径 MUST NOT 将旧顶层 bashExecution/bash_execution 提升为合法上下文（按 agent-session-store s20 跳过该行并可观测 warn）。

  @req:be5 @human
  场景: bash 执行入口
    - System MUST 经 Driver（或等价缝）暴露 bash 执行入口（execute_bash 或等价，含结果记录与中止）；单/双 bang 前缀 MUST 在产品面路由到执行器。

  @req:be6 @human
  场景: 上下文过滤
    - 构建 LLM 上下文（含 ReAct 从 store 播种的 history）时 System MUST 省略 exclude_from_context 为 true 的 bashExecution（Message 内 nested）；单 bang 默认纳入。MUST NOT 依赖旧顶层 BashExecution 读提升。

  @req:b1 @human
  场景: bash trait
    - System MUST 在 BashOperations trait 后抽象 bash 执行，支持真实与 mock 实现供测试。

  @req:b2 @human
  场景: bash hooks
    - System MUST 支持 bash 执行的 pre-spawn 与 post-spawn hooks 以供扩展集成。

  @req:b3 @human
  场景: 超时逐级升级
    - 当调用方提供有限超时时，System MUST 实现逐级超时：先 SIGTERM，5 秒宽限后 SIGKILL。未提供超时时 MUST NOT 仅因默认秒数触发该升级路径。

  @req:be7 @human
  场景: 运行时可达 abort
    - 交互 bash 的取消入口 abort_bash MUST 可从 AgentRuntime::abort（&self）到达，无需独占 &mut AgentCapabilities 才能在 Driver::abort 路径杀进程树；CancellationToken 取消后 MUST 触发既有杀树路径（be2）。

  @req:be8 @human
  场景: 执行器超时可选
    - XyBashExecutor / BashExecOpts MUST 支持可选超时；省略时的默认策略由调用方决定——产品工具层 MUST 传入默认上限，执行器自身 MUST NOT 硬编码秒数；仍可经 CancellationToken 取消。
