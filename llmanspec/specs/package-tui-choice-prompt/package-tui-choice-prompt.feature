# language: zh-CN
# capability: package-tui-choice-prompt
# purpose: 通用 ChoicePrompt：单选/多选、Other 自由输入、多题 Tab 混搭（AskQuestion 形态，供 agent_demo / 产品内联槽）。
# scope: packages/xylitol-tui/

功能: package-tui-choice-prompt

  @req:r1581
  规则: single-and-multi
    ChoicePrompt MUST 支持每题 ChoiceMode::Single（互斥）与 ChoiceMode::Multi（可勾选多项）；提交结果 MUST 区分单值与多值；Single 行视觉 MUST NOT 使用 ●/○ 单选圆点（用 →/反色）。
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
  @req:r1582
  规则: other-free-text
    当题目 allow_other 为真时 MUST 提供 Other 选项；用户 MUST 能经 Tab 聚焦自由输入并在提交中带回自定义文本。
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
  @req:r1583
  规则: multi-question-tabs
    当问卷含多题时 MUST 渲染可切换的题目标签与 Submit，且每题可独立为 Single 或 Multi；单题 MUST NOT 强制显示多题 Tab 条。
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
  @req:r1584
  规则: inline-not-overlay
    ChoicePrompt MUST 可作为 Component 嵌入宿主布局（如 editor 槽替换）；MUST NOT 要求以全屏 Overlay 仪表盘呈现。
    # verified-by: llmanspec/specs/app-tui-ask/app-tui-ask.feature
  @req:r1585
  规则: cancel
    Esc（tui.select.cancel）MUST 取消整份问卷并给出 cancelled 结果；MUST NOT 静默丢弃无回调。
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
  @req:r1586
  规则: skip-status
    Esc/Skip 结束时 ChoiceResult.status MUST 为 Skipped，且 to_ask_payload_json MUST 产出 status=skipped 成功载荷；cancelled MUST 仍为真以兼容 Trust deny。
    # verified-by: llmanspec/specs/app-tui-ask/app-tui-ask.feature
  @req:r1587
  规则: review-unanswered-confirm
    多题 Review 若存在未答题：首次 Enter MUST 提示有 tab 未填写且 MUST NOT 提交或 skip；同提示下再 Enter MUST 以 Skipped 结束；←/切题 MUST 取消该二次确认。
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
  @req:r1588
  规则: optional-description
    option.description MUST 可选；全部缺省时 MUST NOT 打开空的右侧说明栏；有 description 时窄宽在聚焦项下展示、宽屏可右侧说明栏。

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: packages/xylitol-tui/src/components/choice_prompt.rs
