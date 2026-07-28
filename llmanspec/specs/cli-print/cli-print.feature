# language: zh-CN
# managed by llman sdd partition-migrate
功能: cli-print

  @req:r34
  场景: happy
    假如 agent 发出 TextDelta 事件
    当 Print 模式激活
    那么 文本渐进出现在 stdout

  @req:r43
  场景: happy
    假如 agent 依次调用 read 与 bash 工具
    当 Print 模式显示结果
    那么 每个工具显示 [Tool: name] 摘要行

  @req:r49
  场景: thinking-streamed
    假如 model 发出 ThinkingDelta 事件
    当 Print 模式激活
    那么 thinking 文本出现在 stderr 且包裹 <think> 标签，stdout 保持干净

  @req:r49
  场景: embedded-tags
    假如 model 在 ThinkingDelta 内自行发出 <think>...</think> 标签
    当 Print 模式渲染
    那么 不添加第二层 <think> 包裹

  @req:r52
  场景: text-delta-only
    假如 model 对同一内容发出 TextDelta 与 MessageUpdate
    当 Print 模式渲染
    那么 stdout 中每个 token 恰好出现一次

  @req:r55
  场景: error-nonzero-exit
    假如 agent 事件流发出 XyEvent::Error
    当 Print 模式消费该流结束
    那么 进程以非零退出码结束

  @req:r55
  场景: success-zero-exit
    假如 agent 事件流无 Error 且正常结束
    当 Print 模式消费该流结束
    那么 进程以零退出码结束
