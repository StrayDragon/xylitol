# language: zh-CN
# purpose: XyEvent 到 UI 模型的单一翻译缝与 agent 循环生命周期。

功能: app-tui-bridge

  @req:r1173
  规则: xyevent-to-ui-seam
    产品 TUI 须经单一 bridge 缝（apply_xy_event）将 XyEvent 译为 UI-only 模型；渲染层 不得直接 match XyEvent。未知/元数据变体 须降级（tracing）且 不得 panic。

    场景: 元数据事件降级安静地无头
      当 以桥缝注入元数据事件（模型选择与 thinking 档位）
      那么 模型不 panic 且条目与相位不变
  @req:r2957
  规则: plan-mode-bridge
    `planModeChanged` 须经 `apply_xy_event` 单一桥缝映射为 `UiModel.mode_strip`：mode=plan → `plan`；mode=execute 且有 plan_id → `execute · <plan_id>`；mode=execute 无 plan_id → 隐藏。`planDocUpdated` 须被桥缝消费且不得 panic。渲染层 不得直接 match 该事件或自行维护模式状态。

    场景: planModeChanged 经单一桥缝映射
      假如 收到 planModeChanged 事件
      当 apply_xy_event 处理
      那么 UiModel.mode_strip 按 mode 与 plan_id 显示或隐藏

  @req:r1180
  规则: agent-loop-lifecycle
    用户可见忙碌态 须覆盖从提交或 steer 起至 AgentEnd 且 follow-up 队列为空；中间 TurnEnd 不得被当作整轮结束去清空 streaming 尾或复位 idle。

    场景: 忙碌生命周期直到agent结束无头
      当 以桥缝注入 AgentStart 与中间 TurnEnd
      那么 相位保持忙碌且 status 不空
      当 注入带 follow_up 队列的 AgentEnd
      那么 相位仍忙碌且 status 为 Follow-up pending
      当 注入空队列的 AgentEnd
      那么 相位回到 idle
  @req:r1952
  规则: prompt-failure-ends-run
    产品 attach 对 prompt 的非成功业务应答 须结束该次 run 并露出错误，不得当作已开跑而继续等待 AgentEnd。Busy 且 follow-up 为空时收到非中止 Error 须进入 Idle 并清忙碌短词；Error 条目仍保留。

    场景: 提示词业务错误空闲忙碌
      假如 模型已进入忙碌 Working
      当 以桥缝注入非中止 Error 且 follow-up 为空
      那么 相位回到 idle 且保留 Error 条目
  @req:r1181
  规则: steer-followup-events
    bridge 须能消费 QueueUpdate（steer_count / follow_up_count）；display_diff 等工具结果字段 须在 ToolExecutionEnd 路径可被抽取供 transcript 使用。

    场景: 队列计数与显示diff无头
      当 以桥缝注入 QueueUpdate steer=2 follow_up=1
      那么 队列计数同步为 2/1
      当 注入 edit 工具成功结果（JSON 含 display_diff）
      那么 同一条工具行的 display_diff 被填入
  @req:r1174
  规则: tool-intent-fixed-zone
    bridge 收到 MessageUpdate 且 message 含 AgentPart::ToolCall 时 须按 tool id upsert UiEntry::Tool（可缺完整 args）；后续同 id 的 MessageUpdate 须刷新 args_preview；ToolExecutionStart 须 upsert 同一行而非再 push。MessageUpdate 只携带 message 快照，bridge 不得期待独立 text/thinking 字段，也不得把快照写入 streaming 缓冲（防与 TextDelta 重复）。助手 Markdown 路径 不得把 toolCall 当正文。resume/travel 重建路径对同 toolCallId 的 call+result 须复用与本条 upsert 及 ToolExecutionEnd 等价的单行合入语义（细则 att12）。中段重连时 当前回合的工具意图行 须由订阅应答的 partial 快照恢复，不得要求 tape 重放。

    场景: 工具意图upsert单一行无头
      当 以桥缝注入含 ToolCall 的助手快照
      那么 按工具 id 恰好一行且路径进 args_preview
      当 注入同 id 的新快照与 ToolExecutionStart
      那么 行被 upsert 而非再 push
      并且 正文快照不写入 streaming 缓冲
  @req:r1182
  规则: attach-default-inprocess-retained
    产品 TUI 默认 须经产品客户端 attach Host：unary 与下行 须共用同一条 WS /rpc；不得默认同进程直握操作器。POST /rpc 仍为 HTTP 产品入口（JSON-RPC 文本），不得删除。同进程驱动路径 须保留给 print、库嵌入与符合性闸，不得删除。表面切换 须只换客户端实现（seam 稳定），不得为产品 TUI 静默改回同进程直握。

    场景: 远端类型保留
      当 服务端在空闲端口上启动
      并且 检查 Driver 实现
      并且 POST /rpc 调用 host.describe
      那么 应答为产品成功且 id 回显
      当 产品客户端经 WS /rpc 调用 host.describe
      那么 产品 unary 成功且 result 含协议版本
      并且 默认 attach 且同进程驱动路径仍保留给 print 与嵌入
  @req:r1183
  规则: compaction-status-scrollback
    产品 bridge 收到 CompactionStart 时 须将 busy status 设为单行短词 Compacting，并 须向 live scrollback 插入一条 transcript compaction 占位块（标签 [compaction] + Compacting…；与完成块同槽）；收到 CompactionEnd 成功时 须就地将该占位变为完成块（默认折叠：载荷带 tokens_after 时为 Compacted from N → M tokens，缺省落回 Compacted from N tokens；均带展开和弦 to expand；展开后显示 summary），不得再追加全文滚动提示 dump 或叠 compaction complete 滚动提示；失败/aborted 时 须就地变为短失败态；若仍 Busy 须将 status 恢复为 Working；resume/rebuild 时 CompactionEntry 须映射为默认折叠的完成块（无 tokens_after，须落回 N 形词）；展开键 须与 tool 块共用产品面 expand 和弦（Alt+E）；不得用多行 status 或 footer 呈现压缩进度/摘要。CompactionEnd 携带一次性诊断 notice 时，bridge 须以滚动提示（ScrollNotice）一行尾插该诊断文本，不得借此改动 compaction 块折叠态或重复滚动同一条诊断。

    场景: 压缩块生命周期无头
      当 以桥缝注入 CompactionStart
      那么 busy status 为 Compacting 且出现 pending 占位块
      当 注入成功 CompactionEnd（含 summary 与 tokens_before）
      那么 占位就地变为完成块且 status 恢复 Working
      当 注入失败 CompactionEnd
      那么 块变为短失败态
  @req:r1184
  规则: auto-retry-status-scrollback
    产品 bridge 收到 AutoRetryStart 时 须将 busy status 设为单行 Retry attempt/max_retries；收到 AutoRetryEnd 时若 success=false 须向 scrollback 追加失败说明，且若仍处于 Busy 须将 status 恢复为 Working；成功结束时若仍 Busy 亦 须恢复 Working；不得在 status 行堆叠多行重试详情。

    场景: 自动重试状态生命周期无头
      当 以桥缝注入 AutoRetryStart attempt=2 max=5
      那么 busy status 为 Retry 2/5
      当 注入失败 AutoRetryEnd
      那么 滚动提示说明失败且 status 恢复 Working
  @req:r1185
  规则: abort-feedback-aborted-note
    用户 Esc abort agent 流后，产品 bridge/host 须向 live scrollback 追加至多一行 muted/ScrollNotice 文案 Aborted，清除 streaming_thinking 与 streaming_assistant，并将 status 置为 idle；若随后收到同轮已丢弃策略外的 XyEvent::Error aborted，不得再追加重复 Aborted 行（仅与末行去重）；bang Esc 取消 须走块内 (cancelled)，不得冒充 agent Aborted。

    场景: 已中止错误注释去重无头
      当 以桥缝注入流式正文与 aborted 错误
      那么 已流式正文保留且至多一行 aborted 提示且相位 idle
      当 再注入同轮 aborted 错误
      那么 不追加重复 aborted 行
  @req:r1178
  规则: error-kind-branch
    产品 bridge 收到 XyEvent::Error 时 须按 kind 分流：Aborted（或 is_aborted）须走 atb7 的 ScrollNotice/idle 路径，不得追加粘性 UiEntry::Error；其余 kind（Config/Session/Provider/Tool/Message/…）须追加粘性 UiEntry::Error（文案为 message），并 须记录含 error.kind 的日志。不得把非 abort 错误伪装成 Aborted 脚注。

    场景: 错误种类分支粘滞对照中止无头
      当 以桥缝注入 Provider 错误
      那么 追加粘性 Error 行
      当 注入 Aborted 错误
      那么 走 aborted 提示路径而非粘性 Error
  @req:r1186
  规则: bash-ui-entry-block
    产品 bridge/host 须以 UI-only 可着色块表示交互 bang（建议 UiEntry::Bash 或等价），携带 command 与 status（pending/success/error/cancelled）及输出；渲染层 不得再把 bang 长期拼成无语义的裸滚动提示行。

    场景: bash界面块不提示无头
      当 以桥缝创建 pending Bash 块并增量追加输出
      那么 条目为 Bash 块而非裸滚动提示且 status 保持 pending
      当 注入完成收口
      那么 块状态切换为终态
  @req:r1187
  规则: bash-event-append
    产品 bridge 须提供对最后一条 Pending Bash 块的增量追加 API（append_bash_output 或等价），输入为交互 bang 的输出事件（bash/output → 面级 sink）；增量追加期间 status 须保持 pending tint；BashDone 或取消收口时 须经既有 finish/cancelled 路径切换终态；不得每 chunk 新建独立滚动提示行。

    场景: bash块追加于位置
      当 以桥缝创建 pending Bash 块并增量追加输出
      那么 条目为 Bash 块而非裸滚动提示且 status 保持 pending

    场景: bash块关闭经已有完成路径
      当 以桥缝创建 pending Bash 块并增量追加输出
      当 注入完成收口
      那么 块状态切换为终态
  @req:r1175
  规则: quiet-write-edit-success-output
    ToolExecutionEnd 对 write/edit 且 is_error=false 时，若 result 为机器成功 JSON（含 path/success 或 display_diff），bridge 不得把该 JSON 当作默认可见的机器结果墙。edit 成功时 须将 display_diff 合入同一条工具块（不得再 push 独立 UiEntry::Diff）。write 意图流式阶段 须把 args.content 写入该工具块可渲染正文（供 viewport）。is_error=true 时 须保留错误文案供块末查看。

    场景: 写入编辑成功安静输出无头
      当 以桥缝注入 write 意图（含 content）与成功 JSON 结果
      那么 正文进同一工具块且成功 JSON 不外显
      当 注入 edit 成功 JSON 结果（含 display_diff）
      那么 diff 合入同一条工具块且不另起 Diff 行
      当 注入 edit 失败结果
      那么 错误文案保留在块末
  @req:r1176
  规则: write-intent-body-stream
    当 MessageUpdate/ToolCall 名为 write 且 arguments 含 content 时，bridge 须刷新同一 Tool 行的可渲染正文为当前 content 快照（可与 args_preview 并存）；不得等 ToolExecutionEnd 才首次出现正文。

    场景: 写入意图正文之前结束无头
      当 以桥缝在 End 前注入 write 快照（arguments.content）
      那么 工具行正文在 End 前即出现
  @req:r1177
  规则: tool-path-stream-sticky
    write/edit/read 意图流式阶段，args 一旦可解析出 path（含 file_path/file 别名）须立即进入该 Tool 行 args_preview（~/ 缩短）；path 仍空时 须用 … 占位（如 edit … / write …），不得等 ToolExecutionEnd 才首次出现路径槽。同 id 后续 upsert 若新 args 缺 path 但该行已见真实 path，须保留（粘性）。ToolExecutionEnd 不得用空 args 重写已有 call header（bash `$ cmd`、已流式 write/edit path 须在 done 后仍可见）。仅当 header 仍缺真实 path 且成功 JSON 含 path 时 须回填。read 在有 offset/limit 时 须在 path 后附 :start 或 :start-end（end=offset+limit-1；仅 offset 则 :offset）。edit header 不得附加 :N 行域（行号仅在 display_diff 正文）。

    场景: 工具路径流粘滞无头
      当 以桥缝注入 file_path 别名的 write 意图
      那么 路径即进 args_preview
      当 注入缺 path 的后续快照与成功 End
      那么 已见路径保持
      当 注入 read 意图（含 offset 与 limit）
      那么 路径附行号区间
  @req:r1179
  规则: tool-path-wire-parity
    产品 TUI 经 attach 收到的 ToolExecutionStart 须与进程内收到的同一事件使用同一 args 语义；经 wire 往返后，read/write/edit 的 args_preview 须保留可解析出的真实路径，不得因载荷映射丢失而显示为 `Read ...` 等占位。

    场景: 线协议往返保留工具参数无头
      当 经 wire 序列化往返 ToolExecutionStart 后注入桥缝
      那么 args_preview 保留真实路径不退化为占位

  @req:r2935
  规则: 端侧视图与边界
    折叠/展开 须为端侧纯呈现：状态留在客户端（活动折叠状态与滚动折叠表），点击命中与切换全在端侧完成，不得往返 Host，也不得要求 Host 持久化折叠状态。产品 TUI 以 attach 模式运行时 不得直接访问 Host 会话存储，一切会话读取 须经协议 unary（get_messages 及其分页参数）；仅同进程嵌入方 可持有直读端口。会话历史 须按窗口加载：首屏取最新一页，向上滚动逐页向前加载；窗口超出上限时 最旧部分 须回落为顶部可点击提示行，且该提示行 仅在盘上仍有未加载条目时出现。

    场景: 折叠不往返Host
      假如 产品 TUI 会话含可折叠工具行
      当 点击折叠命中区
      那么 折叠状态就地切换且不产生任何 Host unary 往返

    场景: attach模式不直读存储
      假如 产品 TUI 以 attach 模式连接 Host
      当 查询驱动是否暴露会话存储
      那么 不暴露且会话读取仅经协议

    场景: 窗口上限回落为提示行
      假如 会话条目数超过端侧窗口上限
      当 按窗口上限裁剪滚动区
      那么 最旧部分替换为顶部提示行且提示行可点击加载更早
