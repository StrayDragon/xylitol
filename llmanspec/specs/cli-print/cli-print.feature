# language: zh-CN
# capability: cli-print
# purpose: Print 模式（非交互）输出渲染 — TextDelta 流式 stdout、reasoning 流式 stderr 与工具摘要。
# scope: src/app/cli/

功能: cli-print

  @req:r34
  规则: streaming-output
    Print 模式下 System MUST 实时将 agent 文本输出流式写入 stdout。

    场景: print-streams-delta-only
      当 渲染 print 事件流到缓冲
      那么 stdout 恰为增量拼接且无前缀重复
  @req:r43
  规则: tool-display
    Print 模式下 System MUST 显示工具执行名称与结果摘要。

    场景: print-shows-tool-name-and-summary
      当 渲染含工具执行的事件流到缓冲
      那么 工具名与结果摘要按人话格式生成
  @req:r49
  规则: thinking-to-stderr
    Print 模式下 System MUST 将 reasoning/thinking 内容流式写入 stderr（非 stdout），使正式答案不被污染，包裹于 <think>...</think>；模型已自行发出 <think> 标签时 MUST NOT 双重包裹。
    # verified-by: src/app/cli/print.rs
  @req:r52
  规则: message-dedup
    Print 模式下 System MUST 仅将增量 TextDelta 写入 stdout，MUST NOT 将累积 MessageUpdate payload 写入 stdout，以避免前缀重复输出。

    场景: print-no-accumulated-payload
      当 渲染 print 事件流到缓冲
      那么 stdout 恰为增量拼接且无前缀重复
  @req:r55
  规则: error-nonzero-exit
    Print 模式在 XyEvent 流中出现 Error（或等价不可恢复失败）时，进程 MUST 以非零退出码结束；无此类错误且 run 正常结束（含 should_stop_after_turn）时 MUST 以零退出。工具单次失败但 ReAct 继续时 MUST NOT 仅因此非零退出。

    场景: print-error-nonzero-tool-fail-zero
      当 渲染含错误的事件流到缓冲
      那么 错误返回驱动错误而工具单败不退出
      当 渲染含单次工具失败的事件流到缓冲
      那么 单次工具失败不产生驱动错误
