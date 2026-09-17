# language: zh-CN
# capability: package-tui-agent-demo
# purpose: xylitol-tui agent_demo 演示面：command plate 预制触发、瘦固定区、打字机 Markdown stub（≠ 产品 DESIGN playground）。
# scope: packages/xylitol-tui/examples/, packages/xylitol-tui/tests/

功能: package-tui-agent-demo

  @req:r1572 @human
  场景: command-plate-slot
    - agent_demo MUST 提供 command plate（经 Ctrl+P 或等价 slash），以替换 editor 槽的方式展示可选演示项；MUST NOT 做成居中大 overlay 仪表盘。

  @req:r1573 @human
  场景: canned-triggers
    - command plate 每一项 MUST 绑定预制动作或预制 prompt；选中后 MUST 触发可重复的演示脚本（含 Markdown 全语法、流式代码高亮、Diff、工具三态、会话树等已支持场景中的子集）。

  @req:r1574 @human
  场景: slim-default-seed
    - 默认 seed transcript MUST 保持短小（帮助指向 plate）；重型 showcase MUST 经 plate 或显式 slash（如 /md）注入，MUST NOT 仅依赖过长默认 seed。

  @req:r1575 @human
  场景: footer-no-key-wall
    - agent_demo footer MUST NOT 常驻多行或长键位墙；完整键位说明 MUST 经 plate 项或 /help 进入 transcript。

  @req:r1576 @human
  场景: demo-compaction-retry-fixed-zone
    - agent_demo MUST 提供可触发的 Compacting 与 Retry n/m 单行 status 预览（和弦或 plate 或 slash），时序结束后 MUST 回到 Ready/Working 类空闲短词，并 MUST 向 transcript 追加对应 ScrollNotice；MUST NOT 以 capturing overlay focus-restore 演示作为本预览的一部分。

  @req:r1577 @human
  场景: thinking-border-cycle
    - agent_demo MUST 以 Shift+Tab 作为 thinking border level 的主 cycle 入口（可另保留 plate 或 /thinking-level 辅助）；每次 cycle MUST 经 Editor set_border_color（或 apply_thinking_border）应用对应 Palette 边框；bash 模式边框 MUST 仍优先 success，退出 bash 后 MUST 恢复当前 thinking 边框；MUST NOT 将本 cycle 实现为产品 TUI 默认键位（产品 thinking 仅经 /model 槽，见 ati36）。
