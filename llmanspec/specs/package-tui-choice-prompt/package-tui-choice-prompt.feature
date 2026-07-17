# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-choice-prompt

  @req:pcp00
  场景: shell-present
    当 列出 capability package-tui-choice-prompt
    那么 spec 含 pcp01–pcp05

  @req:pcp01
  场景: single-enter
    假如 单题 Single 有两项
    当 高亮第二项并 Enter
    那么 结果含该单值且非 cancelled

  @req:pcp01
  场景: multi-space
    假如 单题 Multi
    当 Space 勾选两项再 Enter
    那么 结果含两个值

  @req:pcp02
  场景: other-tab
    假如 allow_other 真
    当 移到 Other 行按 Tab 输入再 Enter
    那么 结果含自定义文本且标记 was_custom

  @req:pcp03
  场景: tabs-multi-q
    假如 两题问卷
    当 渲染
    那么 可见两题标签与 Submit

  @req:pcp03
  场景: tabs-single-q
    假如 一题问卷
    当 渲染
    那么 无多题 Tab 条

  @req:pcp04
  场景: inline-slot
    假如 agent_demo 打开 ask plate
    当 观察布局
    那么 组件在 editor 槽内联显示

  @req:pcp05
  场景: esc-cancel
    假如 问卷打开
    当 按 Esc
    那么 回调 cancelled 为真
