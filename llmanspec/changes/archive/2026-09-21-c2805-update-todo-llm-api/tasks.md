# Tasks — Todo LLM API（两工具 + 请求时尾插）

垂直切片。接缝全部复用既有入口（见 `research/current-impl-and-seams.md`），
不新发明 harness。

## 接缝（已锁定）

| 接缝 | 入口 |
|---|---|
| 领域 | `src/protocol/session/todo.rs` 单测 |
| 工具 + 网关 | `src/infra/tools/todo.rs` 单测 |
| 9 闭集 / Barrier | `src/infra/tools/mod.rs`；`steps_tools.rs` 烟测 |
| 冻表 | `src/agent/capabilities/mod.rs` `freeze_includes_todo_builtins_first_turn` |
| 投影尾插 | `src/agent/prompt/status_bar.rs`（`AgentStatusBar` + `project_outbound`）；ReAct generate 前；token estimator；`llm_project` 只做 history fold |
| 待办栏 | `src/app/tui/widgets/todo_bar.rs`；BDD `todo-bar-default-shows-in-progress` / `todo-bar-resume-matches-live` |
| 工具块 | `src/app/tui/bridge/preview.rs`；BDD att36 checklist / 空摘要 / 空表 body |
| 事件 | `TodoUpdated` 仍全量；`react/tests.rs`、`wire/event.rs` 往返 |
| compact | `ensure_agent_todo_after_compact`（SSOT 重挂，不尾插摘要请求） |
| BDD 步 | `steps_tools.rs` / `steps_agent.rs` 的「10 个」文案与断言；`steps_app_tui_transcript.rs` list 夹具 |

## T1 领域：三态 + 稳 id + 旧 cancelled 跳过

- [x] `TodoStatus` 去掉 `Cancelled`；`done_total` 只计 completed
- [x] rewrite 省略 id → `t_` + 8 hex，禁止 `todo-{idx}`
- [x] 读快照：`status: "cancelled"` 丢掉该行，其余行仍加载
- [x] 单测：`src/protocol/session/todo.rs`
- 验证：域单测绿；含 cancelled 的夹具仍能 `latest_agent_todo`

## T2 工具面：砍 list、items[] update、guidelines

- [x] 删除对模型暴露的 `todo_list`；闭集 9 名；gateway `list()` 仅内部
- [x] `todo_update` 改为 `items[]`（每条必有 id）；拒绝顶层 `{id,status}`
- [x] mutate 成功仍回全表 `{items}`
- [x] description 短句；`todo_update` 一条 `prompt_guidelines`（点名 `<agent_status_bar>` / `<todo>`）
- [x] description 去掉 dual in_progress
- [x] gateway `update` → patches；成功仍 persist 全量 + `TodoUpdated { list }`
- [x] 单测：`infra/tools/todo.rs`；freeze 只冻 rewrite/update
- [x] BDD：`all-ten-tools-smoke` / `registry` / agent-session 背景步改为 9；烟测不再调 list
- 验证：工具单测 + freeze + `tests::bdd::` agent-tools / agent-session
- [blocked-by: T1]

## T3 出站 AgentStatusBar `<todo>`

- [x] `project_outbound` 追加至多一条 user 行；空表省略；不写 history / JSONL
- [x] XML 根 `<agent_status_bar>`；清单为 `<todo>`；item 属性 id+status，正文为转义后 content
- [x] **MUST NOT** 进 system、**MUST NOT** 并入 session_env
- [x] `agent_todo` Custom 仍 `as_agent_message == None`
- [x] 主 generate 与 token estimator 计入；compact 摘要请求不投影栏
- [x] 单测：`status_bar.rs` 有表则末条含 `<todo>`；无表则投影与今日同形；Custom 不进 `project_for_llm`
- 验证：status_bar + `llm_project` skip-custom + estimator 计入栏
- [blocked-by: T1]

## T4 待办栏：前翼只 completed

- [x] `todo_bar` 不再把 cancelled 混进 past；翼头无 `· n cancelled`
- [x] glyph 表去掉 `[-]`
- [x] 单测：`todo_bar.rs` 含 cancelled 的夹具改为「读取跳过 / 不出现」
- 验证：待办栏单测 + BDD r1129/r1130
- [blocked-by: T1]

## T5 工具块 header：update 补丁计数；空表改 rewrite

- [x] update header：`{n} updated`（args.`items` 是补丁数组）
- [x] rewrite 仍 `N items · M in progress`；流式未齐无 `items` → 空摘要
- [x] body 仍按结果 `{items}` 画 glyph+content（att36）
- [x] 空表 body / 空摘要 BDD 改用 `todo_rewrite` `items: []`，不再用 `todo_list`
- 验证：`preview.rs` 单测 + `todo-summary-empty-not-placeholder` / `todo-block-empty-list-body-hint`
- [blocked-by: T2]

## T6 门禁

- [x] `just fmt` / `just lint` / 相关 `cargo test` / `tests::bdd::` todo / tools / session 场景
- [blocked-by: T3, T4, T5]
