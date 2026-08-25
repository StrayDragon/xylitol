# language: zh-CN
# capability: app-tui-ask
# purpose: 产品 TUI 内置工具 ask：澄清/分叉问卷、仅 TUI 装配、Choice 槽与 scrollback 人话摘要。
# scope: 产品 TUI 面

功能: app-tui-ask

  @req:ata1 @human
  场景: tui-only-registration
    - 产品 TUI 装配的 ToolSet MUST 含内置工具 ask；Print 与非 TUI composition MUST NOT 注册 ask。

  @req:ata2 @human
  场景: choice-slot-mount
    - 当 ask 工具执行等待用户时，产品 TUI MUST 在 editor 槽挂载 ChoicePrompt（解冻 EditorSlot::Choice）；MUST NOT 使用 stderr 数字菜单。

  @req:ata3 @human
  场景: skip-success
    - 用户 Skip 或 Esc MUST 使 ask 以成功结构化结果返回（status=skipped）；MUST NOT 将 skip 表示为 tool error；Abort/取消整轮 MUST NOT 伪装为 skip。

  @req:ata4 @human
  场景: answered-payload
    - 用户提交 MUST 返回 status=answered 与 answers（含 question id 与所选 values/labels）；LLM 消费 JSON；人看 scrollback 人话。

  @req:ata5 @human
  场景: scrollback-ask-rail
    - ask 相关 scrollback MUST 使用固定左边轨与人话摘要（waiting/answered/skipped 语义色）；MUST NOT 默认以 tool-*-bg 洗底或 raw JSON 作为人对人展示。

  @req:ata6 @human
  场景: not-trust
    - ask MUST NOT 替代 Trust 闸；Trust 仍走 app-tui-trust / bootstrap ChoicePrompt。

  @executable @req:ata1
  场景: tui-registers-ask
    假如 产品以 TUI 面启动
    当 查询装配后的 ToolSet
    那么 含名为 ask 的内置工具

  @executable @req:ata1
  场景: print-omits-ask
    假如 产品以 Print 面启动
    当 查询装配后的 ToolSet
    那么 不含名为 ask 的工具

  @executable @req:ata2
  场景: mounts-choice-slot
    假如 ask 工具已开始等待用户
    当 产品 TUI 渲染 editor 槽
    那么 槽为 ChoicePrompt 且标题含 Ask

  @executable @req:ata3
  场景: esc-skips-success
    假如 ChoicePrompt 因 ask 打开
    当 按 Esc
    那么 ask 工具结果为 status skipped 成功 JSON

  @executable @req:ata4
  场景: submit-answered
    假如 ChoicePrompt 因 ask 打开且用户已选选项
    当 提交
    那么 ask 工具结果为 status answered 且含 answers

  @executable @req:ata5
  场景: scrollback-human-rail
    假如 ask 已结束（answered 或 skipped）
    当 渲染 scrollback
    那么 出现 Ask 人话摘要与左边轨且无 raw tool JSON 洗底

  @executable @req:ata6
  场景: trust-untouched
    假如 项目尚未信任
    当 启动需 Trust 闸的路径
    那么 仍走 Trust ChoicePrompt 而非 ask 工具
