# language: zh-CN
# capability: cli-print
# purpose: Print 模式（非交互）输出渲染 — TextDelta 流式 stdout、reasoning 流式 stderr 与工具摘要。
# scope: CLI print 应用面

功能: cli-print

  @req:r34 @human
  场景: streaming-output
    - Print 模式下 System MUST 实时将 agent 文本输出流式写入 stdout。

  @req:r43 @human
  场景: tool-display
    - Print 模式下 System MUST 显示工具执行名称与结果摘要。

  @req:r49 @human
  场景: thinking-to-stderr
    - Print 模式下 System MUST 将 reasoning/thinking 内容流式写入 stderr（非 stdout），使正式答案不被污染，包裹于 <think>...</think>；模型已自行发出 <think> 标签时 MUST NOT 双重包裹。

  @req:r52 @human
  场景: message-dedup
    - Print 模式下 System MUST 仅将增量 TextDelta 写入 stdout，MUST NOT 将累积 MessageUpdate payload 写入 stdout，以避免前缀重复输出。

  @req:r55 @human
  场景: error-nonzero-exit
    - Print 模式在 XyEvent 流中出现 Error（或等价不可恢复失败）时，进程 MUST 以非零退出码结束；无此类错误且 run 正常结束（含 should_stop_after_turn）时 MUST 以零退出。工具单次失败但 ReAct 继续时 MUST NOT 仅因此非零退出。
