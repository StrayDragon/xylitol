# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-transcript

  @req:att1
  场景: markdown-assistant
    当 UiModel 含助手 Markdown 条目并 render
    那么 输出含 Markdown 主题着色而非纯 `assistant:` 前缀行

  @req:att3
  场景: diff-component
    当 UiEntry::Diff 含 display_diff
    那么 render 经包 Diff 着色

  @req:att4
  场景: edit-status-rail
    当 渲染含 display_diff 的成功 edit 工具块
    那么 header 与 Diff 正文共用 success 轨、无整行 tool-success-bg 洗底、且无 diff-*-bg 行底分层

  @req:att5
  场景: reuse-rail-paint
    当 渲染 tool 块
    那么 调用 paint_left_rail_line（或等价包 API）而非手写轨/gutter

  @req:att6
  场景: tree-not-transcript-browser
    当 用户要回看分支
    那么 走双 Esc 会话树而非 TranscriptView

  @req:att7
  场景: fold-key-hint
    当 渲染折叠 thinking 头
    那么 行旁含 (Ctrl+T)

  @req:att8
  场景: alt-e-tools-and-diff
    假如 存在 tool 与 diff 块
    当 Alt+E
    那么 两类块展开态一同翻转

  @req:att8
  场景: hint-parenthesized
    假如 渲染折叠 thinking 头
    当 行旁提示
    那么 含 (Ctrl+T) 而非裸 ^T

  @req:att9
  场景: stream-frames-pending
    假如 bang 执行中已收到至少一帧输出 chunk
    当 渲染 scrollback
    那么 同一块含部分输出且仍为 pending 轨色

  @req:att10
  场景: gap-between-blocks
    假如 scrollback 含相邻 user 与 bang 块
    当 render
    那么 两块内容行之间至少一行空白

  @req:att10
  场景: rail-not-full-wash
    假如 渲染成功 tool 或 bang 块
    当 检查着色行
    那么 可见 status 轨前缀且 MUST NOT 整行铺 tool-success-bg

  @req:att11
  场景: reuse-rail-paint-bang
    假如 渲染 bang 块
    当 调用 paint_left_rail_line（或等价包 API）
    那么 而非手写轨或整行 tool-*-bg 洗底

  @req:att12
  场景: rebuild-keeps-thinking
    假如 assistant SessionEntry content 含 thinking 与 text
    当 rebuild_scrollback_from_travel 或 fork 后重建
    那么 UiModel 含独立 Thinking 与 Assistant 条目

  @req:att12
  场景: rebuild-idempotent-with-live
    假如 同一轮直播已 flush 出 Thinking+Assistant
    当 用其 persist 的 SessionEntry 再 rebuild
    那么 分块种类与顺序一致（正文等价）

  @req:att12
  场景: rebuild-tool-call-result-merged
    假如 路径含 assistant toolCall 与同 toolCallId 的 toolResult
    当 rebuild_scrollback_from_travel
    那么 UiModel 对该 toolCallId 恰好一条 done UiEntry::Tool（含 args_preview 与 output）

  @req:att12
  场景: rebuild-tool-idempotent-with-live
    假如 同一轮直播已 ToolExecutionEnd 成单块 Tool
    当 用其 persist 的 SessionEntry 再 rebuild
    那么 Tool 条数与 id/preview/output/done 与直播末态幂等

  @req:att18
  场景: nav-notice-appended
    假如 travel 重建完成且含 history @ 通知
    当 检查 UiModel.entries
    那么 该通知不在 entries 首条而在路径投影之后
