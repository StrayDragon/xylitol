# Tasks

## 1. Specs landing（合约）

- [x] 1.1 工作区干净后 Branch binding：`llman sdd change start c1850-add-tui-ask-tool`
- [x] 1.2 新建 `app-tui-ask`：`spec.toon` + `app-tui-ask.feature`（仅 TUI 注册、槽、skip/answered、rail 人话、≠ Trust）
- [x] 1.3 扩展 `package-tui-choice-prompt`：Skip 成功语义、Review 未答二次确认、description 可选（文档场景或单测锚点）
- [x] 1.4 扩展 `agent-tools`：内置 `ask` schema/结果；Print 不注册；Barrier 并发类
- [x] 1.5 `llman sdd validate` + commit Specs landing

## 2. 包合约收口

- [x] 2.1 对齐 ChoicePrompt 与 1.3（若 demo 已超前则补测试/导出缺口）[blocked-by: 1.3]
- [x] 2.2 `just test-tui` 相关 choice_prompt / agent_demo ask plates 绿 [blocked-by: 2.1]

## 3. 产品接线（垂直切片）

- [x] 3.1 实现 `ask` 工具（infra）+ TUI-only 注入 ToolSet [blocked-by: 1.4]
- [x] 3.2 解冻 `EditorSlot::Choice`：挂 ChoicePrompt；Skip/提交回灌 oneshot [blocked-by: 3.1, 2.1]
- [x] 3.3 scrollback Ask 块：轨色 + 人话摘要；禁 tool wash/JSON 默认 [blocked-by: 3.2]
- [x] 3.4 产品 BDD / 单测覆盖 1.2 场景 [blocked-by: 3.3, 1.2]

## 4. 设计与对照

- [x] 4.1 同步 `ask.md` / DESIGN 索引与实现偏差（若有）
- [x] 4.2 确认 Trust 路径未改；demo `/entry-style` 仍仅 demo [blocked-by: 3.3]

## 5. 门禁

- [x] 5.1 `just qa`（或 lint + test + test-tui + 相关 BDD）[blocked-by: 3.4, 2.2]
  - 已跑：`just fmt` / `just lint`；`cargo test --lib ask`；`cargo test -p xylitol-tui choice_prompt|ask`；`cargo test --test bdd -- app_tui_ask`；composition reload_preserves_tui_ask；满闸 `cargo test --lib` 有既有并行 keybindings/config flake，与本 change 无关。
