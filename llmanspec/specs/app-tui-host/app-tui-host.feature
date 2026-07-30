# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-host

  @req:ath1
  场景: no-start
    当 审计产品 TUI 源码
    那么 无 TUI::start；存在 host 驱动调用

  @req:ath2
  场景: panic-restore
    当 TUI 运行中发生 panic
    那么 panic hook 恢复终端后用户 shell 可用

  @req:ath3
  场景: debug-tail
    当 以 debug 构建启动 TUI
    那么 日志文件持续写入且 AGENTS 记载路径

  @req:ath4
  场景: tiny-terminal
    当 终端高度为 3
    那么 显示放大提示或干净退出，进程不挂死

  @req:ath5
  场景: harness-min-size
    假如 TestTerminal 尺寸为 20x3
    当 HostSession 推进一帧
    那么 视口含放大提示且进程逻辑不 panic

  @req:ath5
  场景: harness-no-start
    假如 产品 TUI 源码与 host 单元测试
    当 审计调用点
    那么 产品路径无 TUI::start 调用

  @req:ath6
  场景: one-pump
    假如 生产 host loop 与 harness 泵均接线
    当 审查 drain_pending 调用点
    那么 两侧共用同一函数且无重复的 PendingSlash match 副本

  @req:ath7
  场景: one-select
    假如 审查 run_host_loop 与 harness bang 路径
    当 存在进行中的 bang
    那么 两侧共用同一 helper/扇入；无第三份手写 bang Esc select

  @req:ath7
  场景: esc-during-stream
    假如 hanging bang 已 uplink 且有 chunk
    当 按 Esc
    那么 Driver::abort 被调用且块呈 cancelled

  @req:ath8
  场景: abort-then-deltas-ignored
    假如 busy 已 Esc abort
    当 再泵入 ThinkingDelta 与 TextDelta 与 AgentEnd
    那么 无 assistant 正文条目且 status idle

  @req:ath8
  场景: esc-xy-before-drain
    假如 agent busy
    当 Esc 后在 drain_pending 前注入 TextDelta
    那么 该 delta 被丢弃且 UI 不回 busy

  @req:ath10
  场景: layout-section-present
    假如 打开 src/app/tui/AGENTS.md
    当 审计文档结构
    那么 含文件布局表、模块职责、硬约束与验证指针，且无进度表

  @req:ath9
  场景: shared-bang-helper
    假如 生产 mod.rs bang 环与 harness bang Esc 测并存
    当 审查 bang Esc 消费路径
    那么 两侧调用同一共享 helper 或同序同分支入口，无第三份手写 select

  @req:ath9
  场景: bdd-features-present
    假如 tests/features 含 app-tui abort/bang/esc/queue feature
    当 cargo test --test bdd -- --test-threads=1
    那么 新增 app-tui 场景全部通过且既有核心 BDD 不回归

  @req:ath11
  场景: bang-abort-unchanged
    假如 hanging bang 中 Esc
    当 取消
    那么 块呈 cancelled 且无 ScrollNotice Aborted 混用

  @req:ath12
  场景: god-files-under-budget
    假如 拆分后统计行数
    当 审查 host/mod layout/root effects 入口 bridge/mod
    那么 各文件显著低于约 800 行且无第二套 drain 泵

  @req:ath12
  场景: input-policy-module
    假如 审查 host 目录
    当 定位 busy Esc 与 idle Enter
    那么 位于 input_policy（或等价）子模块且 step 仍为唯一调度入口

  @req:ath12
  场景: effects-slash-split
    假如 审查 effects 目录
    当 定位 PendingSlash 臂
    那么 按族分文件且 harness 仍调用同一 drain_pending

  @req:ath13
  场景: get-messages-error-note
    假如 Driver get_messages 返回 Err
    当 执行 travel
    那么 scrollback 含失败提示且不假装空树成功

  @req:ath14
  场景: list-via-driver
    假如 产品 harness 或单元
    当 打开 resume 列表路径
    那么 经 Driver/seam 返回且无 tui→infra::session 依赖

  @req:ath14
  场景: row-has-preview-fields
    假如 list_sessions 返回非空
    当 检查行字段
    那么 含预览或 name 以及可用于 age 的时间信息

  @req:ath15
  场景: new-via-driver
    假如 产品 harness
    当 提交 /session-new
    那么 经 Driver new_session 返回新 id 且无 tui→infra::session 依赖

  @req:ath15
  场景: name-via-driver
    假如 产品 harness
    当 提交 /session-name demo
    那么 经 Driver set_session_name 且无 tui 直写目录

  @req:ath16
  场景: delete-rejects-current
    假如 Resume 面板选中当前活跃 session
    当 触发删除确认并确认
    那么 出现拒绝提示且会话仍存在

  @req:ath16
  场景: rename-via-driver
    假如 Resume 面板已开
    当 完成一项 rename
    那么 Driver set_session_name 被调用且列表展示新名

  @req:ath20
  场景: skills-catalog-refresh
    假如 idle 且磁盘 skills 变更后
    当 /reload
    那么 dollar skill catalog 或 loaded_skill_names 反映新目录

  @req:ath20
  场景: partial-success
    假如 某一 reload 步骤失败
    当 /reload
    那么 系统报告含失败项且其它步骤仍执行

  @req:ath21
  场景: seam-only
    假如 ScriptedDriver 可观测
    当 /history-copy-last 成功
    那么 copy_text 调用记录含目标正文

  @req:ath21
  场景: skip-non-assistant
    假如 末条为 system 其前为 assistant
    当 /history-copy-last
    那么 复制的是 assistant 而非 system

  @req:ath22
  场景: silent-no-transcript
    假如 产品 host 已挂载
    当 经 /model 提交新 thinking level
    那么 scrollback 无 thinking-border 系统行且 footer 与边框与 Driver 一致

  @req:ath22
  场景: model-switch-resync
    假如 已切换到支持集不同的模型且 level 被设为最高档
    当 下一帧 render
    那么 footer 与边框反映 Driver::thinking_level 结果

  @req:ath22
  场景: busy-pending-no-system
    假如 agent busy 且 active 模型为 A
    当 有参 /model B 成功更新 selected
    那么 scrollback 无 model → 滚动提示确认行且 status 下轮预告含 Next turn

  @req:ath23
  场景: reload-refreshes
    假如 idle 且磁盘 skills 变更后
    当 /reload
    那么 loaded-resources 与 dollar skill catalog 反映新目录

  @req:ath23
  场景: no-infra-reach
    假如 审查 app/tui 源码
    当 检查 import
    那么 无 tui→infra::mcp 直达

  @req:ath23
  场景: startup-mcp-nonblocking-new-and-resume
    假如 配置了 MCP 且选择新会话或 CLI --session resume
    当 进入 TUI
    那么 面已渲染（resume 时历史已可投影）且未等待全部 MCP 连接完成

  @req:ath23
  场景: prompt-allowed-while-mcp-connecting
    假如 MCP 仍 connecting
    当 提交普通 agent prompt
    那么 允许提交；头卡仍可显示 connecting 进度；slash 与滚历史仍可用

  @req:ath23
  场景: in-tui-resume-no-mcp-reconnect-block
    假如 MCP 已连接或仍在后台连接
    当 面内 /session-resume 切换会话
    那么 不因切换而阻塞等待 MCP 重连

  @req:ath27
  场景: mcp-slash-opens-select-list-any-state
    假如 MCP 已配置且可能仍 connecting 或 agent busy
    当 提交无参 /mcp
    那么 打开替换 editor 槽的 MCP SelectList 且 MUST NOT 仅因开面板 abort agent

  @req:ath27
  场景: mcp-select-list-shows-armed
    假如 至少一个 MCP 已 settle 并 overlay 进 ToolSet
    当 打开 /mcp
    那么 对应行显示 tools armed（或等价）且汇总可区分 armed

  @req:ath27
  场景: mcp-select-list-enter-closes
    假如 /mcp SelectList 已打开
    当 按 Enter
    那么 关槽且 MUST NOT 假实现禁用 MCP

  @req:ath27
  场景: mcp-short-cue-fixed-copy
    假如 配置了多个 MCP 且尚未全部 armed
    当 渲染可选短 cue
    那么 cue 文案为 mcp pending (see /mcp) 且 MUST NOT 含分数计数或 server id 列表

  @req:ath24
  场景: idle-tick-skips-paint
    假如 Ready 且 idle 无 paint_dirty
    当 HostEvent::Tick
    那么 不因该 Tick 单独 request_render

  @req:ath24
  场景: bang-chunk-paints-on-tick
    假如 bang 追加 chunk 已标 paint_dirty
    当 随后 Tick
    那么 合并 request_render 且 scrollback 可见新输出

  @req:ath24
  场景: spinner-reuses-upper-cache
    假如 busy 且 transcript 很长
    当 仅 status Loader 推进或仅 status 短词变化
    那么 UiRoot 复用上区缓存且 spinner 帧仍更新

  @req:ath25
  场景: scrollback-entry-cache-under-streaming
    假如 已有多条已提交 Assistant 条目
    当 连续注入多次 TextDelta
    那么 不得对全部历史条目重新 Markdown；harness 可观测的重绘或 cache miss 有上界

  @req:ath25
  场景: large-scrollback-e2e-smoke
    假如 PTY 或 tmux 下产品 TUI 带较大 scrollback 模拟
    当 完成一轮 Fake 对话并 /exit
    那么 进程干净退出且不得超时挂死

  @req:ath26
  场景: streaming-assistant-reuses-stable-prefix
    假如 Busy 且 streaming_assistant 已有多段完整段落
    当 连续追加多次 TextDelta 仅增长后缀
    那么 全量 Markdown 解析次数有上界且可见行与全量解析一致

  @req:ath26
  场景: streaming-paint-preserves-ctrl-o-viewport
    假如 工具块处于高度缩略
    当 流式 assistant 增量绘制进行中并切换 Ctrl+O
    那么 工具块视口展开/折叠语义不变

  @req:avs1
  场景: h2-stream
    假如 HostSession 已 on_run_started
    当 注入 TextDelta 与 AgentEnd
    那么 scrollback 含助手文本且 phase 为 Idle

  @req:avs1
  场景: h3-tool
    假如 Busy 中
    当 注入 ToolExecutionStart/End
    那么 存在 UiEntry::Tool 且渲染含工具名

  @req:avs1
  场景: h4-steer
    假如 Busy 且 editor 非空
    当 薄编排处理 Enter
    那么 Driver::steer 被记录且出现 Steering: strip

  @req:avs1
  场景: h7-abort-resume
    假如 Busy 中 Esc 已 abort
    当 再 idle 提交
    那么 第二次 Driver::run 被调用且无粘性 aborted

  @req:avs1
  场景: h8-exit
    假如 idle 输入 /exit
    当 编排收尾
    那么 session quit 且 finish_inline/stop 可观测

  @req:avs2
  场景: pty-hello-exit
    假如 PTY 下 Fake+--trust 产品 TUI 已就绪
    当 提交短 prompt 再 /exit
    那么 屏含 Hello from fake provider 且进程退出
