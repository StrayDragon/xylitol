# language: zh-CN
功能: app-tui-ask

  @req:ata1
  场景: tui-registers-ask
    假如 产品以 TUI 面启动
    当 查询装配后的 ToolSet
    那么 含名为 ask 的内置工具

  @req:ata1
  场景: print-omits-ask
    假如 产品以 Print 面启动
    当 查询装配后的 ToolSet
    那么 不含名为 ask 的工具

  @req:ata2
  场景: mounts-choice-slot
    假如 ask 工具已开始等待用户
    当 产品 TUI 渲染 editor 槽
    那么 槽为 ChoicePrompt 且标题含 Ask

  @req:ata3
  场景: esc-skips-success
    假如 ChoicePrompt 因 ask 打开
    当 按 Esc
    那么 ask 工具结果为 status skipped 成功 JSON

  @req:ata4
  场景: submit-answered
    假如 ChoicePrompt 因 ask 打开且用户已选选项
    当 提交
    那么 ask 工具结果为 status answered 且含 answers

  @req:ata5
  场景: scrollback-human-rail
    假如 ask 已结束（answered 或 skipped）
    当 渲染 scrollback
    那么 出现 Ask 人话摘要与左边轨且无 raw tool JSON 洗底

  @req:ata6
  场景: trust-untouched
    假如 项目尚未信任
    当 启动需 Trust 闸的路径
    那么 仍走 Trust ChoicePrompt 而非 ask 工具
