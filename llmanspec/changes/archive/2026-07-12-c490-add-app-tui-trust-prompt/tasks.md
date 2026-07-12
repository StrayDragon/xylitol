# Tasks — c490-add-app-tui-trust-prompt

- [x] 1. Bootstrap：TUI 路径 Ask → `NeedsTrust` / pending，MUST NOT 调 `prompt_trust_options_stdio`
- [x] 2. UiRoot / host：editor 槽挂 `ChoicePrompt`（dark theme）；Esc=deny；提交写 trust store
- [x] 3. 决完后完成项目资源加载并进入 idle chrome（空 editor）
- [x] 4. 单测 / harness：pending → select Trust → store 写入；Esc → 不信任
- [x] 5. 更新 `design/trust-prompt.md`（去「草稿」；指向本实现）
- [x] 6. `llman sdd validate c490-add-app-tui-trust-prompt --strict --no-interactive`
- [x] 7. `just qa`（相关 app tui + trust 测）
