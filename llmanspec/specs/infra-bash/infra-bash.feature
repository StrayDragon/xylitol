# language: zh-CN
# capability: infra-bash
# purpose: 经 shell 子进程执行 Bash 命令，含流式输出与取消。
# scope: src/infra/bash_exec/, src/protocol/

功能: infra-bash

  @req:r1429
  规则: 流式执行器
    System MUST 提供 XyBashExecutor，经 BashExecOpts 执行 shell 命令，携带可选 CancellationToken 与可选有界 mpsc chunk_tx 输出字节；chunk_tx 为 Some 时执行器 MUST 在该通道发送输出块（Full 时发送方合并）；为 None 时 MUST 静默累积且不要求回调。原 on_chunk 回调签名 MUST NOT 保留为公共端口。
    # verified-by: src/infra/bash_exec/mod.rs
    场景: echo-via-executor
      假如 bash 工具即将执行 echo hello
      当 执行 bash echo hello
      那么 返回 output:hello 且未触发临时文件落盘

  @req:r1430
  规则: 中止与取消
    BashExecutor MUST 支持取消，杀死整个进程组并将结果标为 cancelled。

    场景: bang-abort-cancels-executor
      假如 启动交互 bang 长命令后 abort
      当 检查 bash 结果
      那么 cancelled 为 true
  @req:r1431
  规则: 输出截断
    BashExecutor MUST 将输出截断到配置 max bytes，超限时将完整输出溢出到临时文件；返回给调用方/会话的 output MUST 为截断尾部，并 MUST 追加 pi 形脚注 `[Full output: <path>. Truncated: <N> lines shown (<limit> limit)]`（path 不可用时用 `(unavailable)`）；未截断 MUST NOT 追加该脚注。
    # verified-by: src/infra/bash_exec/mod.rs
    场景: overflow-to-temp-file
      假如 bash 输出在上限边界以不完整 UTF-8 序列结束
      当 调用 truncate_output
      那么 输出在字符边界安全截断且不 panic

  @req:r1432
  规则: 会话条目
    新写入的 bang-bash MUST 持久化为 SessionEntry::Message（type=message），其 message 为 Env bashExecution（role=bashExecution），携带 command、output、exit_code、cancelled、truncated、full_output_path、exclude_from_context。MUST NOT 再对新写入使用顶层 type=bashExecution 变体。读路径 MUST NOT 将旧顶层 bashExecution/bash_execution 提升为合法上下文（按 agent-session-store s20 跳过该行并可观测 warn）。

    场景: bang-execution-recorded-as-message
      假如 存在会话 "fmt-bash"
      当 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
      那么 bash 记录行 type 为 message 且 role 为 bashExecution
  @req:r1433
  规则: bash 执行入口
    System MUST 经 Driver（或等价缝）暴露 bash 执行入口（execute_bash 或等价，含结果记录与中止）；单/双 bang 前缀 MUST 在产品面路由到执行器。

    场景: idle-bang-routes-to-executor
      当 以主机泵在 idle 提交 bang 命令
      那么 execute_bash 收到命令体且未调用 run
  @req:r1434
  规则: 上下文过滤
    构建 LLM 上下文（含 ReAct 从 store 播种的 history）时 System MUST 省略 exclude_from_context 为 true 的 bashExecution（Message 内 nested）；单 bang 默认纳入。MUST NOT 依赖旧顶层 BashExecution 读提升。

    场景: excluded-bash-not-in-context
      当 以压后上下文检查 bash 排除
      那么 排除的 bash 条目不进上下文而普通条目保留
  @req:r1426
  规则: bash trait
    System MUST 在 BashOperations trait 后抽象 bash 执行，支持真实与 mock 实现供测试。
    # verified-by: src/AGENTS.md
    场景: trait-abstraction-mockable
      假如 构建无 bash executor 的 agent
      当 调用 execute_bash
      那么 返回提及 bash executor 未配置的错误且不 panic

  @req:r1427
  规则: bash hooks
    System MUST 支持 bash 执行的 pre-spawn 与 post-spawn hooks 以供扩展集成。

    场景: bash-before-hook-rejects
      假如 注册匹配 bash 的 before 拒绝 hook
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且未执行
  @req:r1428
  规则: 超时逐级升级
    当调用方提供有限超时时，System MUST 实现逐级超时：先 SIGTERM，5 秒宽限后 SIGKILL。未提供超时时 MUST NOT 仅因默认秒数触发该升级路径。
    # verified-by: src/infra/bash_exec/mod.rs
    场景: escalating-timeout-kills
      假如 bash 运行 yes 命令
      当 stdout 超过 1MB 上限
      那么 子进程被杀并返回截断输出

  @req:r1435
  规则: 运行时可达 abort
    交互 bash 的取消入口 abort_bash MUST 可从 AgentRuntime::abort（&self）到达，无需独占 &mut AgentCapabilities 才能在 Driver::abort 路径杀进程树；CancellationToken 取消后 MUST 触发既有杀树路径（be2）。
    # verified-by: llmanspec/specs/infra-bash/infra-bash.feature
    场景: abort-reachable-from-runtime
      假如 启动交互 bang 长命令后 abort
      当 检查 bash 结果
      那么 cancelled 为 true

  @req:r1436
  规则: 执行器超时可选
    XyBashExecutor / BashExecOpts MUST 支持可选超时；省略时的默认策略由调用方决定——产品工具层 MUST 传入默认上限，执行器自身 MUST NOT 硬编码秒数；仍可经 CancellationToken 取消。
    # verified-by: src/infra/bash_exec/mod.rs
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）
    场景: optional-timeout-and-missing-arg
      当 调用bash 不传命令参数
      那么 调用失败且返回 MissingArgument 错误码
