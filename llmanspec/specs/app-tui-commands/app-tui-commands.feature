# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-commands

  @req:atm1
  场景: model-opens-picker
    假如 idle 且会话树关闭
    当 Enter 提交无参 /model
    那么 打开 models 槽且不调用 CycleModel

  @req:atm1
  场景: model-id-after-completion
    假如 idle 编辑器经补全得到 /model fake
    当 Enter 提交
    那么 SetModel 或等价被调用且 thinking 为该模型支持集最高档或 off

  @req:atm1
  场景: picker-level-keys
    假如 Models 槽已打开且焦点模型可调 thinking
    当 按 ←→ 或槽内 Shift+Tab
    那么 焦点模型的预览档在支持集内前进或后退

  @req:atm1
  场景: no-thinking-dash
    假如 Models 槽焦点为无思考模型
    当 按 ←→ 或槽内 Shift+Tab
    那么 等级区保持 — 且状态不变

  @req:atm2
  场景: model-via-dispatch
    当 用户输入 /model
    那么 经 dispatch 或等价 Driver 方法切换模型

  @req:atm3
  场景: unknown-slash
    当 用户输入 /not-a-command
    那么 显示内联错误且会话继续

  @req:atm4
  场景: model-bare-not-cycle
    假如 产品 harness idle
    当 /model Enter
    那么 pending 为开 picker 而非 CycleModel

  @req:atm5
  场景: list-scenes
    假如 产品 idle
    当 提交 /debug
    那么 系统提示含 session-tree-multiturn 或 session-tree-labeled 及描述

  @req:atm5
  场景: load-multiturn
    假如 产品 idle
    当 提交 /debug session-tree-multiturn
    那么 当前 session 变为 debug- 前缀且 transcript/树可含 fixture user 正文

  @req:atm5
  场景: arg-completion
    假如 产品 debug 构建 idle 且已键入 /debug 加空格
    当 补全 popup 出现场景 id
    那么 可 Tab/Enter 写入 editor

  @req:atm6
  场景: slash-session-tree-opens
    假如 产品 idle
    当 提交 /session-tree
    那么 EditorSlot::Tree 打开且含 Search 或 Type to search

  @req:atm6
  场景: slash-session-fork-leaf
    假如 产品 idle 且 session 有 leaf
    当 提交 /session-fork
    那么 fork+switch 发生或 ScriptedDriver fork 被调用

  @req:atm6
  场景: old-tree-fork-unknown
    假如 产品 idle
    当 提交 /tree 或 /fork
    那么 显示 unknown command 类系统错误且不开树不 fork

  @req:atm7
  场景: catalog-ssot
    假如 审查 slash_catalog 与 agent 产品 SSOT
    当 比对 name 集合
    那么 一致（debug-only 命令可仅 debug 构建出现）

  @req:atm7
  场景: legacy-unknown
    假如 产品 idle Editor
    当 提交 /tree
    那么 unknown 系统行且不打开树

  @req:atm8
  场景: compact-bare
    假如 产品 idle
    当 提交 /session-compact
    那么 Compact 经 dispatch 被调用或 ScriptedDriver compact 被调用

  @req:atm8
  场景: compact-rejects-args
    假如 产品 idle
    当 提交 /session-compact please
    那么 出现 usage 类错误且未 compact

  @req:atm8
  场景: export-default-html
    假如 产品 idle
    当 提交无参 /session-export
    那么 走 ExportHtml 或等价且系统行含导出路径

  @req:atm8
  场景: export-jsonl-suffix
    假如 产品 idle
    当 提交 /session-export /tmp/out.jsonl
    那么 走 ExportJsonl 或等价

  @req:atm8
  场景: import-confirm-cancel
    假如 产品 idle
    当 提交 /session-import /tmp/a.jsonl 后在确认槽选 No 或 Esc
    那么 未 import 且有取消提示

  @req:atm9
  场景: session-dumps-stats
    假如 产品 idle
    当 提交 /session
    那么 scrollback 或系统行含 session id 或消息统计字段

  @req:atm9
  场景: session-rejects-args
    假如 产品 idle
    当 提交 /session info
    那么 出现错误或不识别为 dump 成功路径

  @req:atm10
  场景: resume-opens-panel
    假如 产品 idle
    当 提交 /session-resume
    那么 editor 槽出现含 scope 或 Sort 或搜索提示的 Resume 面板

  @req:atm10
  场景: resume-switch
    假如 产品 idle 且列表有项
    当 在面板 Enter 选定
    那么 SwitchSession 发生且 transcript 刷新

  @req:atm10
  场景: resume-search-regex
    假如 面板已开且有多项
    当 输入 re: 可匹配某预览的模式
    那么 可见行收窄为匹配项

  @req:atm10
  场景: old-resume-unknown
    假如 产品 idle
    当 提交 /resume
    那么 显示 unknown command 类错误

  @req:atm10
  场景: resume-id-hidden-by-default
    假如 产品 idle 且列表含带 uuid 的会话
    当 提交 /session-resume 打开面板
    那么 会话行可见预览与 meta 且默认不展示完整 session id

  @req:atm10
  场景: resume-ctrl-u-shows-full-id
    假如 Resume 面板已开且 id 列隐藏
    当 按 Ctrl+U
    那么 会话行出现完整 session id 且未被截断

  @req:atm11
  场景: session-new-bare
    假如 产品 idle
    当 提交 /session-new
    那么 Driver new_session 被调用且 transcript 清空或系统行提示新会话

  @req:atm11
  场景: session-new-rejects-args
    假如 产品 idle
    当 提交 /session-new x
    那么 出现 usage 类错误且未 new_session

  @req:atm11
  场景: session-clone-at-leaf
    假如 产品 idle 且有 leaf
    当 提交 /session-clone
    那么 fork_session 以 At 调用并 switch

  @req:atm11
  场景: session-clone-no-leaf
    假如 产品 idle 且无 leaf
    当 提交 /session-clone
    那么 提示无 leaf 且未 fork

  @req:atm11
  场景: session-name-set
    假如 产品 idle
    当 提交 /session-name hello
    那么 set_session_name 被调用或系统行含最终名

  @req:atm11
  场景: old-lifecycle-unknown
    假如 产品 idle
    当 提交 /new 或 /clone 或 /name
    那么 显示 unknown command 类错误

  @req:atm12
  场景: idle-reload
    假如 产品 idle 且 foundation 可测
    当 提交 /reload
    那么 系统块含 reload 或各资源成功/失败摘要且历史条目数不变

  @req:atm12
  场景: busy-refuse
    假如 产品 busy
    当 提交 /reload
    那么 拒绝提示且未改历史

  @req:atm12
  场景: catalog-lists
    当 打开 slash 补全
    那么 列表含 reload

  @req:atm13
  场景: idle-trust
    假如 产品 idle
    当 提交 /trust
    那么 系统块含信任已保存与 /reload 或 restart 提示且未调用 reload_runtime

  @req:atm13
  场景: busy-refuse
    假如 产品 busy
    当 提交 /trust
    那么 拒绝提示且未写盘

  @req:atm13
  场景: catalog-lists
    当 打开 slash 补全
    那么 列表含 trust

  @req:atm13
  场景: arg-complete
    假如 产品 idle 编辑器
    当 输入 /trust 加空格
    那么 补全列表含 self parent deny 且默认可应用 self

  @req:atm13
  场景: usage-unknown
    假如 产品 idle
    当 提交 /trust foo
    那么 usage 错误系统块

  @req:atm14
  场景: copy-ok
    假如 transcript 含 assistant
    当 提交 /history-copy-last
    那么 系统块成功提示且 Driver copy 被调用

  @req:atm14
  场景: copy-empty
    假如 无 assistant 条目
    当 提交 /history-copy-last
    那么 系统块提示无可复制内容且未伪造成功

  @req:atm14
  场景: busy-allowed
    假如 产品 busy 且有 assistant
    当 提交 /history-copy-last
    那么 仍执行复制（非 busy 拒绝）

  @req:atm14
  场景: catalog-lists
    当 打开 slash 补全
    那么 列表含 history-copy-last

  @req:atm15
  场景: bare-opens-slot
    假如 idle
    当 提交无参 /theme
    那么 打开 Themes 槽且列表含 dark 与 light

  @req:atm15
  场景: named-light
    假如 idle 且当前 dark
    当 提交 /theme light
    那么 应用 light 且系统块成功提示

  @req:atm15
  场景: toggle
    假如 idle 且 preference 为 dark
    当 提交 /theme toggle
    那么 应用 light

  @req:atm15
  场景: busy-reject
    假如 产品 busy
    当 提交 /theme light
    那么 系统块拒绝且主题未变

  @req:atm15
  场景: catalog-lists
    当 打开 slash 补全
    那么 列表含 theme
