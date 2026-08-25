# language: zh-CN
# capability: app-tui-bridge
# purpose: XyEvent 到 UI 模型的单一翻译缝与 agent 循环生命周期。
# scope: 产品 TUI 面

功能: app-tui-bridge

  @req:atb1 @human
  场景: xyevent-to-ui-seam
    - 产品 TUI MUST 经单一 bridge 缝（apply_xy_event）将 XyEvent 译为 UI-only 模型；渲染层 MUST NOT 直接 match XyEvent。未知/元数据变体 MUST 降级（tracing）且 MUST NOT panic。

  @req:atb2 @human
  场景: agent-loop-lifecycle
    - 用户可见忙碌态 MUST 覆盖从提交或 steer 起至 AgentEnd 且 follow-up 队列为空；中间 TurnEnd MUST NOT 被当作整轮结束去清空 streaming 尾或复位 idle。

  @req:atb3 @human
  场景: steer-followup-events
    - bridge MUST 能消费 QueueUpdate（steer_count / follow_up_count）；display_diff 等工具结果字段 MUST 在 ToolExecutionEnd 路径可被抽取供 transcript 使用。

  @req:atb10 @human
  场景: tool-intent-chrome
    - bridge 收到 MessageUpdate 且 message 含 AgentPart::ToolCall 时 MUST 按 tool id upsert UiEntry::Tool（可缺完整 args）；后续同 id 的 MessageUpdate MUST 刷新 args_preview；ToolExecutionStart MUST upsert 同一行而非再 push。MessageUpdate 的 text/thinking 快照 MUST NOT 写入 streaming 缓冲（防与 TextDelta 重复）。助手 Markdown 路径 MUST NOT 把 toolCall 当正文。resume/travel 重建路径对同 toolCallId 的 call+result MUST 复用与本条 upsert 及 ToolExecutionEnd 等价的单行合入语义（细则 att12）。

  @req:atb4 @human
  场景: attach-default-inprocess-retained
    - 产品 TUI 默认 MUST 经四象限客户端 attach Host；MUST NOT 默认同进程直握操作器。同进程驱动路径 MUST 保留给 print、库嵌入与符合性闸，MUST NOT 删除。表面切换 MUST 只换客户端实现（seam 稳定），MUST NOT 为产品 TUI 静默改回同进程直握。

  @req:atb5 @human
  场景: compaction-status-scrollback
    - 产品 bridge 收到 CompactionStart 时 MUST 将 busy status 设为单行短词 Compacting，并 MUST 向 live scrollback 插入一条 transcript compaction 占位块（标签 [compaction] + Compacting…；与完成块同槽）；收到 CompactionEnd 成功时 MUST 就地将该占位变为完成块（默认折叠：Compacted from N tokens（展开和弦 to expand）；展开后显示 summary），MUST NOT 再追加全文滚动提示 dump 或叠 compaction complete 滚动提示；失败/aborted 时 MUST 就地变为短失败态；若仍 Busy MUST 将 status 恢复为 Working；resume/rebuild 时 CompactionEntry MUST 映射为默认折叠的完成块；展开键 MUST 与 tool 块共用产品面 expand 和弦（Alt+E）；MUST NOT 用多行 status 或 footer 呈现压缩进度/摘要。

  @req:atb6 @human
  场景: auto-retry-status-scrollback
    - 产品 bridge 收到 AutoRetryStart 时 MUST 将 busy status 设为单行 Retry attempt/max_retries；收到 AutoRetryEnd 时若 success=false MUST 向 scrollback 追加失败说明，且若仍处于 Busy MUST 将 status 恢复为 Working；成功结束时若仍 Busy 亦 MUST 恢复 Working；MUST NOT 在 status 行堆叠多行重试详情。

  @req:atb7 @human
  场景: abort-feedback-aborted-note
    - 用户 Esc abort agent 流后，产品 bridge/host MUST 向 live scrollback 追加至多一行 muted/ScrollNotice 文案 Aborted，清除 streaming_thinking 与 streaming_assistant，并将 status 置为 idle；若随后收到同轮已丢弃策略外的 XyEvent::Error aborted，MUST NOT 再追加重复 Aborted 行（仅与末行去重）；bang Esc 取消 MUST 走块内 (cancelled)，MUST NOT 冒充 agent Aborted。

  @req:atb14 @human
  场景: error-kind-branch
    - 产品 bridge 收到 XyEvent::Error 时 MUST 按 kind 分流：Aborted（或 is_aborted）MUST 走 atb7 的 ScrollNotice/idle 路径，MUST NOT 追加粘性 UiEntry::Error；其余 kind（Config/Session/Provider/Tool/Message/…）MUST 追加粘性 UiEntry::Error（文案为 message），并 MUST 记录含 error.kind 的日志。MUST NOT 把非 abort 错误伪装成 Aborted 脚注。

  @req:atb8 @human
  场景: bash-ui-entry-block
    - 产品 bridge/host MUST 以 UI-only 可着色块表示交互 bang（建议 UiEntry::Bash 或等价），携带 command 与 status（pending/success/error/cancelled）及输出；渲染层 MUST NOT 再把 bang 长期拼成无语义的裸滚动提示行。

  @req:atb9 @human
  场景: bash-stream-append
    - 产品 bridge MUST 提供对最后一条 Pending Bash 块的增量追加 API（append_bash_output 或等价）；流式 chunk 追加期间 status MUST 保持 pending tint；BashDone 或取消收口时 MUST 经既有 finish/cancelled 路径切换终态；MUST NOT 每 chunk 新建独立滚动提示行。

  @req:atb11 @human
  场景: quiet-write-edit-success-output
    - ToolExecutionEnd 对 write/edit 且 is_error=false 时，若 result 为机器成功 JSON（含 path/success 或 display_diff），bridge MUST NOT 把该 JSON 当作默认可见的机器结果墙。edit 成功时 MUST 将 display_diff 合入同一条工具块（MUST NOT 再 push 独立 UiEntry::Diff）。write 意图流式阶段 MUST 把 args.content 写入该工具块可渲染正文（供 viewport）。is_error=true 时 MUST 保留错误文案供块末查看。

  @req:atb12 @human
  场景: write-intent-body-stream
    - 当 MessageUpdate/ToolCall 名为 write 且 arguments 含 content 时，bridge MUST 刷新同一 Tool 行的可渲染正文为当前 content 快照（可与 args_preview 并存）；MUST NOT 等 ToolExecutionEnd 才首次出现正文。

  @req:atb13 @human
  场景: tool-path-stream-sticky
    - write/edit/read 意图流式阶段，args 一旦可解析出 path（含 file_path/file 别名）MUST 立即进入该 Tool 行 args_preview（~/ 缩短）；path 仍空时 MUST 用 … 占位（如 edit … / write …），MUST NOT 等 ToolExecutionEnd 才首次出现路径槽。同 id 后续 upsert 若新 args 缺 path 但该行已见真实 path，MUST 保留（粘性）。ToolExecutionEnd MUST NOT 用空 args 重写已有 call header（bash `$ cmd`、已流式 write/edit path MUST 在 done 后仍可见）。仅当 header 仍缺真实 path 且成功 JSON 含 path 时 MUST 回填。read 在有 offset/limit 时 MUST 在 path 后附 :start 或 :start-end（end=offset+limit-1；仅 offset 则 :offset）。edit header MUST NOT 附加 :N 行域（行号仅在 display_diff 正文）。

  @req:atb15 @human
  场景: tool-path-wire-parity
    - 产品 TUI 经 attach 收到的 ToolExecutionStart MUST 与进程内收到的同一事件使用同一 args 语义；经 wire 往返后，read/write/edit 的 args_preview MUST 保留可解析出的真实路径，MUST NOT 因载荷映射丢失而显示为 `Read ...` 等占位。

  @executable @req:atb4
  场景: remote-type-kept
    当 检查 Driver 实现
    那么 默认 attach 且同进程驱动路径仍保留给 print 与嵌入
