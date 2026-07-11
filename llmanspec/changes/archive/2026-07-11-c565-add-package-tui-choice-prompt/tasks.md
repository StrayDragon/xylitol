# Tasks — c565-add-package-tui-choice-prompt

- [x] 1. Delta `package-tui-choice-prompt`（pcp00–pcp05）通过 `llman sdd validate c565-add-package-tui-choice-prompt --strict --no-interactive`
- [x] 2. playground 新增 Ask 槽（Single / Multi / Tabs / Other 聚焦态示意）+ README 一行
- [x] 3. 实现 `ChoicePrompt`（Single/Multi、Other+Tab 聚焦、多题 Tab、Esc cancel）+ 导出
- [x] 4. 组件单测：单选提交、多选 Space、Other Tab、多题切换、Esc
- [x] 5. `agent_demo` plates `ask-single` / `ask-multi` / `ask-tabs` + harness 断言
- [x] 6. `cargo test -p xylitol-tui choice_prompt` 与相关 `agent_demo_test`；validate 再跑一遍
