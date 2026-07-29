# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-bridge

  @req:atb1
  场景: unknown-event
    当 收到未映射 XyEvent
    那么 记录日志且 UI 不崩溃

  @req:atb1
  场景: no-xyevent-in-render
    当 审查 tui 渲染模块
    那么 无 XyEvent 匹配；事件经 apply_xy_event

  @req:atb2
  场景: tool-turn-middle
    当 工具调用后出现中间 TurnEnd
    那么 UI 仍保持 busy 并继续消费后续事件

  @req:atb3
  场景: queue-update
    当 steer 与 follow-up 入队
    那么 UI 收到 QueueUpdate 计数

  @req:atb4
  场景: remote-type-kept
    当 检查 Driver 实现
    那么 InProcess 为默认且 RemoteDriver 仍存在

  @req:atb5
  场景: compact-start-status
    假如 agent Busy
    当 CompactionStart
    那么 status 为 Compacting 且 scrollback 含 compaction 占位块

  @req:atb5
  场景: compact-end-restores-working
    假如 Busy 且 status=Compacting 且已有占位块
    当 CompactionEnd aborted=false 且含 summary 与 tokens_before
    那么 占位就地变为默认折叠完成块且 status 恢复 Working
    并且 MUST NOT 追加 compaction complete System 行

  @req:atb5
  场景: compact-end-aborted
    假如 Busy 且 Compacting 且已有占位块
    当 CompactionEnd aborted=true
    那么 占位就地变为短失败态且 status 恢复 Working

  @req:atb5
  场景: compact-rebuild-collapsed
    假如 session 含 CompactionEntry
    当 rebuild scrollback from travel
    那么 出现默认折叠的 compaction 完成块
    并且 MUST NOT 整段 System dump summary

  @req:atb6
  场景: retry-start-status
    假如 agent Busy
    当 AutoRetryStart attempt=2 max=5
    那么 status 为 Retry 2/5

  @req:atb6
  场景: retry-end-fail-note
    假如 Busy 且 Retry 状态
    当 AutoRetryEnd success=false
    那么 scrollback 含失败说明且 status 恢复 Working

  @req:atb6
  场景: retry-end-success-restores
    假如 Busy 且 Retry 状态
    当 AutoRetryEnd success=true
    那么 status 恢复 Working 且无失败 System 行

  @req:atb7
  场景: esc-clears-streaming
    假如 busy 且已有 streaming_thinking
    当 note_user_abort
    那么 streaming 缓冲为空且含 Aborted

  @req:atb8
  场景: bash-entry-shape
    假如 idle 提交 bang
    当 审查 UiModel entries
    那么 存在 Bash 块或等价含 command 字段

  @req:atb9
  场景: append-keeps-pending
    假如 已 begin_bash_block
    当 连续两次 append_bash_output
    那么 同一 Bash 条目 output 增长且 status 仍为 pending
