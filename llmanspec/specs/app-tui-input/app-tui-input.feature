# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-input

  @req:ati1
  场景: selector-slot
    当 打开模型选择器
    那么 editor 槽被选择器替换且贴底可见

  @req:ati2
  场景: esc-aborts
    当 流式中按 Esc
    那么 Driver::abort 被调用

  @req:ati3
  场景: alt-enter-followup
    当 忙碌时 Alt+Enter 提交文本
    那么 消息进入 follow-up 队列而非立即 steer

  @req:ati4
  场景: no-tool-modal
    当 信任目录下工具执行
    那么 无逐工具审批 UI 且 hooks 仍可注册

  @req:ati5
  场景: esc-closes-tree
    假如 会话树已打开
    当 按 Esc
    那么 树关闭且 editor 槽恢复

  @req:ati6
  场景: busy-enter-steer
    假如 agent_demo 轮次忙碌且编辑器有文本
    当 按 Enter
    那么 steer 队列长度增加且状态仍为忙碌类（Thinking/Working/Drafting 等）

  @req:ati6
  场景: alt-enter-followup
    假如 agent_demo 轮次忙碌
    当 Alt+Enter 提交文本
    那么 follow-up 队列长度增加且会话树尚未出现该 follow-up 的 user 节点

  @req:ati6
  场景: footer-counts
    假如 steer 或 follow-up 队列非空
    当 渲染 footer
    那么 footer 含 steer:N 或 follow-up:N

  @req:ati7
  场景: steer-no-abort
    假如 已 submit 进入 Thinking 且 scheduled_actions 非空
    当 再次 Enter 提交 steer 文本
    那么 scheduled 流式/工具动作仍可继续推进而非被立即清空为仅 Ready

  @req:ati8
  场景: bang-enables-bash
    假如 编辑器文本为 !ls
    当 sync 边框
    那么 bash_mode 为真

  @req:ati8
  场景: clear-bang-restores
    假如 曾处于 bash 模式
    当 文本改为 hello 后 sync
    那么 bash_mode 为假

  @req:ati9
  场景: ctrl-g-stub
    假如 编辑器有文本
    当 Ctrl+G 或 stub API
    那么 external_editor 计数加一且 transcript 含 external editor stub

  @req:ati10
  场景: busy-enter-steer
    假如 产品 host 忙碌且 editor 非空
    当 按 Enter
    那么 steer 被调用或 pending steer 入队且 editor 清空

  @req:ati10
  场景: busy-alt-enter-followup
    假如 产品 host 忙碌
    当 Alt+Enter 提交文本
    那么 follow_up 被调用或 pending follow-up 入队

  @req:ati10
  场景: busy-esc-abort
    假如 产品 host 忙碌且树关闭
    当 按 Esc
    那么 abort 被请求且不打开会话树

  @req:ati11
  场景: busy-steer-strip
    假如 产品 host 忙碌且已入队 steer
    当 渲染一帧
    那么 出现 Steering: 行且无 scrollback [steer] System 墙

  @req:ati11
  场景: busy-followup-strip
    假如 产品 host 忙碌且已入队 follow-up
    当 渲染一帧
    那么 出现 Follow-up: 行与 Alt+Up hint

  @req:ati11
  场景: busy-alt-up-dequeue
    假如 产品 host 忙碌且队列非空
    当 按 Alt+Up
    那么 队列文本进入 editor 且请求 clear 两侧队列

  @req:ati12
  场景: steer-inject-scrollback
    假如 busy 且 steer 已注入
    当 收到 user MessageStart
    那么 scrollback 出现对应用户行

  @req:ati12
  场景: followup-inject-scrollback
    假如 follow-up 注入后 QueueUpdate 清 queue strip
    当 渲染
    那么 Follow-up 文本仍在 UiEntry::User 中

  @req:ati13
  场景: submit-then-up
    假如 idle 提交过非空 prompt 且 editor 已空
    当 按 ↑
    那么 editor 回填该 prompt

  @req:ati13
  场景: steer-in-history
    假如 busy 发送过 steer 文本
    当 idle 后 editor 空再按 ↑
    那么 可召回该 steer 文本

  @req:ati14
  场景: esc-then-submit
    假如 TUI busy 中 Esc abort 后
    当 再 idle Enter 提交
    那么 新一轮开始且无连续 aborted Error 墙

  @req:ati15
  场景: clear-restores-thinking
    假如 曾处于 bash 边框且 thinking 为 low
    当 文本改为 hello 后 sync
    那么 边框恢复为 low thinking 色而非仅 muted

  @req:ati16
  场景: bang-executes
    假如 idle 且 editor 为 !echo hi
    当 Enter
    那么 execute_bash 被调用且 run 未被调用

  @req:ati16
  场景: bangbang-exclude
    假如 idle 且 editor 为 !!echo x
    当 Enter
    那么 exclude_from_context 为 true

  @req:ati16
  场景: empty-bang
    假如 idle 且 editor 仅为 !
    当 Enter
    那么 出现提示且未执行 bash

  @req:ati16
  场景: slash-wins
    假如 idle 且 editor 为 /exit
    当 Enter
    那么 走 slash 退出而非 bash

  @req:ati17
  场景: ctrl-g-harness-stub
    假如 产品 host 可测/harness 环境
    当 Ctrl+G
    那么 不 spawn 真实编辑器且有 stub 或系统提示可观测且不崩

  @req:ati17
  场景: ctrl-g-missing-editor-error
    假如 交互路径且 VISUAL 与 EDITOR 均未设置或为空
    当 Ctrl+G
    那么 出现 UiEntry::Error 短行提示需配置编辑器且 Editor 原文保留且进程不崩

  @req:ati17
  场景: ctrl-g-spawn-fail-error
    假如 已配置编辑器但 spawn 或读写 tempfile 失败
    当 Ctrl+G 真路径执行
    那么 出现 UiEntry::Error 短行且 Editor 原文保留且不崩

  @req:ati17
  场景: ctrl-g-tty-success-writeback
    假如 交互 TTY 且 VISUAL 或 EDITOR 已配置且编辑器成功退出
    当 Ctrl+G
    那么 经 with_terminal_suspended 往返后 Editor 文本为编辑器保存内容

  @req:ati18
  场景: tree-slot-live
    假如 产品 TUI idle
    当 双 Esc 打开 Tree 槽
    那么 槽内为 MessageHistory 活树且 Esc 可关回 Editor

  @req:ati19
  场景: esc-cancels-hanging-bang
    假如 产品 idle bang 已开始且 bash 未返回
    当 按 Esc
    那么 Driver::abort 被调用且 bash 结果 cancelled 或等价中止

  @req:ati20
  场景: second-bang-rejected
    假如 第一 bang 仍在执行
    当 再提交 !echo second
    那么 出现拒绝提示且 execute_bash 调用次数仍为 1

  @req:ati21
  场景: open-filter-select
    假如 idle 已打开 models 槽
    当 过滤后 Enter 选定一项
    那么 footer model 更新且槽关闭回到 Editor

  @req:ati21
  场景: esc-cancels
    假如 models 槽已打开
    当 Esc
    那么 关槽且模型不变

  @req:ati23
  场景: space-opens-catalog
    假如 产品 idle 且 catalog 已注入
    当 键入 /model 加空格
    那么 补全 popup 出现模型项

  @req:ati23
  场景: tab-applies-id
    假如 /model dee 补全已开
    当 按 Tab
    那么 editor 含 /model 与选定 id

  @req:ati23
  场景: bare-still-slot
    假如 产品 idle
    当 提交无参 /model
    那么 打开 Models 槽而非仅依赖 arg bare catalog

  @req:ati22
  场景: ctrl-o-cycles
    假如 树开且当前 default
    当 按 Ctrl+O
    那么 进入 no-tools 或状态含 [no-tools]

  @req:ati22
  场景: tree-ctrl-t-not-thinking
    假如 树开
    当 按 Ctrl+T
    那么 走 filter toggle 而非 thinking 折叠

  @req:ati22
  场景: esc-clears-search-first
    假如 树开且已键入搜索串
    当 按 Esc
    那么 搜索清空且树仍开；再 Esc 关树

  @req:ati24
  场景: tree-forwards-fold-chords
    假如 产品树已打开
    当 按 ctrl+left
    那么 TreeSelector 收到输入（fold 或分支跳转生效）

  @req:ati25
  场景: shift-f-in-tree
    假如 EditorSlot::Tree 打开
    当 按 Shift+F
    那么 pending fork 或等价入口被置位且不向 editor 写入字面 F

  @req:ati26
  场景: cycle-backward
    假如 树开且当前 FilterMode 为 default
    当 按 Ctrl+Shift+O
    那么 进入 all 或状态含 [all]

  @req:ati22
  场景: ctrl-o-forward-unchanged
    假如 树开且当前 default
    当 按 Ctrl+O
    那么 进入 no-tools 或状态含 [no-tools]

  @req:ati27
  场景: shift-l-opens-edit
    假如 EditorSlot::Tree 打开
    当 按 Shift+L
    那么 进入 label 编辑且树仍开

  @req:ati28
  场景: idle-session-tree-slash
    假如 产品 idle Editor 槽
    当 提交 /session-tree
    那么 pending 为 OpenTree 或树被打开

  @req:ati28
  场景: old-tree-not-pending
    假如 产品 idle Editor 槽
    当 提交 /tree
    那么 不产生 OpenTree pending（unknown 或系统错误）

  @req:ati30
  场景: abort-and-late-delta
    假如 agent busy 且用户 Esc abort
    当 再注入迟到 TextDelta 或 AgentEnd
    那么 UI 保持 Aborted/idle 语义且无新 assistant 正文复活

  @req:ati30
  场景: bang-esc-and-second
    假如 hanging bang 进行中
    当 Esc 取消后再开第二 bang 并可再 Esc
    那么 出现 cancelled 语义且第二 bang 仍可 abort，无粘性 Aborted 挡住

  @req:ati30
  场景: esc-overlay-vs-tree
    假如 busy 无 overlay 或已开 Models/Tree overlay
    当 按 Esc
    那么 无 overlay 时 abort 且不开树；有 overlay 时只关槽不 abort

  @req:ati30
  场景: queue-keys
    假如 agent busy
    当 Enter / Alt+Enter / Alt+Up
    那么 分别触发 steer、follow_up、双队列清空并还原 editor 文本契约

  @req:ati31
  场景: harness-esc-inject-before-drain
    假如 ScriptedDriver 轮次 busy
    当 Esc 后注入 TextDelta 再 drain_pending
    那么 abort 被调用、无新 assistant 正文、可再 submit

  @req:ati32
  场景: agent-busy-bang-rejected
    假如 agent busy 且编辑器为 !ls
    当 按 Enter
    那么 出现拒绝提示、steer 未被调用、编辑器仍含命令

  @req:ati29
  场景: tab-toggles-scope
    假如 Resume 面板已开
    当 按 Tab
    那么 header scope 在 Current 与 All 间切换或列表按 cwd 过滤变化

  @req:ati29
  场景: sort-cycles
    假如 Resume 面板已开
    当 按 Ctrl+S
    那么 Sort 标签在 Threaded/Recent/Fuzzy 间变化

  @req:ati29
  场景: named-filter
    假如 Resume 面板已开且存在未命名会话
    当 按 Ctrl+N 至 Named
    那么 未命名行不可见

  @req:ati29
  场景: delete-confirm-esc
    假如 Resume 面板选中可删项并 Ctrl+D
    当 按 Esc
    那么 未删除且退出确认态

  @req:ati29
  场景: fold-hides-children
    假如 Threaded 且选中有子会话的父节点
    当 按 fold 键
    那么 子行不可见且再 unfold 恢复

  @req:ati33
  场景: at-opens-popup
    假如 产品 idle 且 AtPath 根目录含 hello.rs
    当 键入 @hel
    那么 补全 popup 出现 hello.rs

  @req:ati33
  场景: tab-inserts-path
    假如 @ 补全已开且选中 hello.rs
    当 按 Tab
    那么 editor 含 @ 与 hello.rs 路径引用

  @req:ati34
  场景: idle-submit-expanded
    假如 产品 idle 且 editor 显示长粘贴折叠 marker
    当 Enter 提交
    那么 pending submit 或 Driver::run 文本为展开全文而非 [paste #

  @req:ati34
  场景: steer-expanded
    假如 产品 busy 且 editor 含折叠 paste marker
    当 Enter 入队 steer
    那么 steer 文本为展开全文

  @req:ati35
  场景: match-by-app-id
    假如 产品 idle 且默认绑定
    当 按下 app.thinking.toggle 默认键
    那么 thinking 折叠切换生效

  @req:ati35
  场景: reload-success
    假如 磁盘 keybindings.json 将某 app.* 改到新和弦
    当 调用 reload
    那么 新和弦生效且旧和弦不再触发该动作

  @req:ati35
  场景: reload-bad-keeps-old
    假如 磁盘 JSON 损坏
    当 调用 reload
    那么 绑定保持重载前状态且有诊断或错误提示

  @req:ati40
  场景: completion
    假如 目录含 demo
    当 键入 $de
    那么 补全列表含 demo

  @req:ati40
  场景: highlight
    假如 scrollback 有用户行含 $demo
    当 渲染用户消息
    那么 $demo 使用 skill-ref 前景（可测 ANSI/cell）

  @req:ati36
  场景: no-global-shift-tab-cycle
    假如 产品 idle 且 Editor 槽（非 Models）且模型支持 off 与 high
    当 按 Shift+Tab
    那么 thinking level MUST NOT 仅因该键前进一档

  @req:ati36
  场景: busy-no-global-cycle
    假如 产品 agent busy 且非 Models 槽
    当 按 Shift+Tab
    那么 thinking level MUST NOT cycle 且 MUST NOT 因 cycle 写系统块

  @req:ati37
  场景: paste-inserts-abs-path
    假如 ScriptedDriver 剪贴板含 PNG
    当 触发粘贴图片键
    那么 Editor 文本含 tempfile 绝对路径且无超长 base64

  @req:ati37
  场景: submit-stays-text
    假如 已粘贴图路径且 idle
    当 Enter 提交
    那么 Driver::run 收到含该路径的纯文本且无强制 user Image part

  @req:ati37
  场景: clipboard-text-fallback
    假如 剪贴板无图但有 UTF-8 文本
    当 触发粘贴键
    那么 Editor 插入该文本且 MUST NOT 出现 clipboard no image Error

  @req:ati37
  场景: clipboard-miss-error
    假如 剪贴板无图且无文本
    当 触发粘贴键
    那么 出现短 Error 提示且进程不崩
