# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-agent-demo

  @req:pad1
  场景: open-plate
    当 用户按下 Ctrl+P
    那么 editor 槽被 plate 列表替换且可上下选择

  @req:pad1
  场景: not-overlay-dashboard
    当 打开 plate
    那么 内容仍在 editor 槽区域而非居中 overlay 仪表盘

  @req:pad2
  场景: md-trigger
    当 选中 markdown 全语法项或提交 /md
    那么 transcript 出现全语法 Markdown 助手块

  @req:pad2
  场景: stream-trigger
    当 选中定点流式语言项
    那么 开始脚本化流式回复且含该语言代码块

  @req:pad3
  场景: slim-seed
    当 启动 demo 默认 seed
    那么 viewport/scrollback 以短帮助为主且可通过 plate 再注入 showcase

  @req:pad4
  场景: footer-short
    当 渲染 footer
    那么 单行 dim 元数据为主且无常驻长快捷键清单

  @req:pad5
  场景: compact-status-sequence
    假如 agent_demo 空闲
    当 触发 compact-status
    那么 status 曾为 Compacting 且稍后为 Ready

  @req:pad5
  场景: retry-status-sequence
    假如 agent_demo 空闲
    当 触发 retry-status
    那么 status 曾为 Retry 1/3 且稍后为 Ready

  @req:pad5
  场景: no-overlay-focus-demo
    假如 agent_demo 入口
    当 查阅 plate/slash/和弦
    那么 无 overlay-focus 演示入口

  @req:pad6
  场景: cycle-changes-border
    假如 agent_demo 非 bash
    当 触发 thinking-level cycle
    那么 当前 ThinkingBorderLevel 前进且 editor 边框色相对 cycle 前改变

  @req:pad6
  场景: shift-tab-cycles
    假如 agent_demo 非 bash 且无 overlay 占键
    当 按 Shift+Tab
    那么 ThinkingBorderLevel 前进且边框色改变

  @req:pad6
  场景: bash-overrides
    假如 thinking 边框已为 High 且进入 bash 模式
    当 观察 editor 边框
    那么 为 success 边框而非 High

  @req:pad6
  场景: restore-after-bash
    假如 bash 结束后仍为原 thinking level
    当 观察 editor 边框
    那么 恢复为该 level 的 thinking 边框
