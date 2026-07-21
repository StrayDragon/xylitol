# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-bash

  @req:be1
  场景: stream-via-channel
    假如 chunk_tx 为 Some
    当 命令打印多行
    那么 接收者在 execute 返回前收到覆盖输出的一个或多个字节块

  @req:be1
  场景: no-callback-port
    假如 审查 runtime_protocol bash 端口
    当 查找 on_chunk 回调签名
    那么 公共 execute 仅接受 BashExecOpts 而无 FnMut on_chunk

  @req:be2
  场景: abort
    假如 长时间命令执行中
    当 调用 abort
    那么 进程组被杀且 cancelled 为 true

  @req:be3
  场景: truncate
    假如 输出超过 max bytes
    当 执行完成
    那么 truncated 为 true 且 full_output_path 已设置
    并且 output 含 Full output 脚注与 lines shown

  @req:be4
  场景: entry
    假如 记录 bash 结果
    当 append 到 session
    那么 条目为 type=message 且 message.role=bashExecution

  @req:be4
  场景: legacy-top-level-bash-read
    假如 session 文件含旧顶层 type=bashExecution
    当 加载并构建上下文
    那么 可见等价 bashExecution 消息（字段保留）

  @req:be5
  场景: bang-include
    假如 用户输入以单 bang 开头
    当 命令运行
    那么 结果记录 exclude_from_context false 并送 LLM

  @req:be5
  场景: bang-exclude
    假如 用户输入以双 bang 开头
    当 命令运行
    那么 结果记录 exclude_from_context true 且从上下文省略

  @req:be6
  场景: omit
    假如 history 有 exclude_from_context true 的 bashExecution（Message 内或读提升）
    当 构建 LLM 上下文或 ReAct 播种
    那么 该条目被省略

  @req:b1
  场景: mock
    假如 注册 mock BashOperations
    当 调用 execute_bash
    那么 运行 mock 实现而非真实 shell

  @req:b2
  场景: pre-hook
    假如 已注册 pre-spawn hook
    当 执行 bash 命令
    那么 spawn 前以命令字符串调用 hook

  @req:b3
  场景: timeout
    假如 命令超过超时
    当 以 timeout 调用 execute_bash
    那么 进程先收 SIGTERM 宽限后 SIGKILL

  @req:be7
  场景: runtime-abort-kills
    假如 BashExecHandler 持有进行中的 cancel token
    当 仅通过 &self abort_bash / AgentRuntime::abort 取消
    那么 进程组被杀且 cancelled 为 true
