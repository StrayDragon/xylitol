# c2805：Todo 现行实现与测试接缝

> **基线快照（apply 前）**。现行合约与代码以 live `agent-todo.feature` 与 `src/` 为准：两工具、三态、`t_` id、无 `todo_list`、出站 `<agent_status_bar><todo>`。勿按本文回改。
>
> 调研日期：2026-09-21。当时真值源：代码 + live specs；本文件不修改 specs 或应用代码。

## 1. 领域模型（`src/protocol/session/todo.rs`）

**形状**：`TodoItem` 三字段 `id` / `content` / `status`（`40:45:src/protocol/session/todo.rs`）；`TodoList` 仅 `items: Vec<TodoItem>`（`47:51`）；`TodoStatus` 闭集四态 `pending|in_progress|completed|cancelled`（`26:31`）。`CUSTOM_TYPE_AGENT_TODO = "agent_todo"`（`14`）；载荷 `{"items":[…]}` 经 `to_data_value` / `from_data_value`（`77:84`）。

**校验** `validate_todo_list`（`114:135`）：id trim 非空；content trim 非空；content ≤ `TODO_CONTENT_MAX_CHARS`（80 标量，`16`）。**无**「至多一条 in_progress」域规则；`dual_in_progress_is_allowed` 单测显式允许（`258:264`）。

**rewrite id 生成** `normalize_rewrite_items`（`147:164`）：`id` 缺省或空 → `todo-{idx+1}`（1-based）；content trim；默认 status `pending`（`TodoItemDraft`，`167:177`）。

**latest-wins** `latest_agent_todo`（`137:145`）：leaf 分支逆序扫描 `custom_type == agent_todo` 的首条 Custom。

**todo_update** `apply_todo_update`（`179:204`）：至少改 `status` 或 `content` 之一；未知 id 拒；改后再 `validate_todo_list`。

**compact 重挂**（非 todo.rs 内）：压缩裁切后若 context 窗内无 `agent_todo`，`ensure_agent_todo_after_compact` 将裁切前 latest 快照再 append（`446:475:src/agent/compaction/mod.rs`；单测 `1334:1387`）。

**域单测**（`206:304`）：`domain_status_closed_set_roundtrip`、`latest_wins_full_scan`、`dual_in_progress_is_allowed`、`empty_content_rejects`、`content_over_eighty_scalars_rejects`、`content_exactly_eighty_scalars_ok`、`update_unknown_id_rejects`、`summary_line_counts_done`。

## 2. Gateway 端口 vs 工具层

**端口** `AgentTodoGateway`（`9:25:src/protocol/ports/todo.rs`）：`list` / `rewrite(Vec<TodoItemDraft>)` / `update(id, status?, content?)`；均返回完整 `TodoList`。

**产品实现** `SessionAgentTodoGateway`（`22:96:src/infra/tools/todo.rs`）：`load_current` → `latest_agent_todo`；`persist` → `append_session_entry` + `to_custom_entry_shell`（`58:68`）。未 bind session 时走内存 fallback（`46:49`）。

**工具返回 JSON**：三工具成功均 `list_json` → `serde_json` 序列化 `{"items":[…]}`（`98:100`、`150`、`228-230`、`296-298`）。`todo_list` 只读、**不**发 `TodoUpdated`（`366:416` 单测）。

**描述与 spec 漂移**（工具 `description()`，非域校验）：
- `todo_rewrite`：`"At most one item may be in_progress."`（`179:180`）— 与 r1126 / 域 `dual_in_progress_is_allowed` 矛盾。
- `todo_update`：`"Rejects unknown id or dual in_progress."`（`261:262`）— 后半句过时；`dual_in_progress_rewrite_persists` 单测证明 rewrite 可双 in_progress（`471:501`）。

**BDD 烟测用** `MemoryTodoGateway`（`42:90:src/infra/tools/mod.rs`）；产品组合 `default_tools_with_todo` + `SessionAgentTodoGateway`（`100:114`、`112:115:src/app/core/driver/in_process/mod.rs`）。

## 3. 事件：`TodoUpdated` 与 wire

**领域** `XyEvent::TodoUpdated { list: TodoList }` — **全量快照**，空表 = 清除（`188:195:src/protocol/lifecycle.rs`）。

**发布点**：仅 `todo_rewrite` / `todo_update` 成功路径 `ctx.publish_state`（`229`、`297:src/infra/tools/todo.rs`）；经 `tool_exec` 进入 run stream（`106:src/agent/runtime/tool_exec.rs`）。

**wire**：`to_wire_event` / 反序列化均映射 `Event::TodoUpdated { list }`（`226`、`380:src/protocol/wire/event.rs`）；wire-visible（`106:111`、`119`）。

**spec 约束**：r1122 要求类型化 **完整** TodoList，client 待办栏不得靠解析工具结果（`60:62:llmanspec/specs/agent-todo/agent-todo.feature`）；r1718 要求 wire 可表达同载荷、旧端可忽略（`72:74:llmanspec/specs/protocol-app/protocol-app.feature`）。

**TUI 消费** `apply_tools_family` → `sync_todo_checklist`（`14:18:src/app/tui/bridge/handlers/tools.rs`；`90:92:src/app/tui/bridge/session_tree.rs`）。

## 4. TUI 投影

**待办栏** `todo_bar.rs`：输入 `TodoBarParams.list`；分区仅用 `status` + `content`（`134:160`），**不绘制 id**。glyph 表 `todo_status_glyph`（`12:18:src/app/tui/bridge/preview.rs`）。

**UiModel** 字段 `todo: TodoList`（`253:src/app/tui/bridge/model.rs`）；`HostSession` 经 `layout/root` 读 `ui_model.todo` 渲染 dock（`31:55:src/app/tui/layout/root/render.rs`）。

**transcript args_preview** `human_tool_args_preview_with_path`（`205:224:src/app/tui/bridge/preview.rs`）：
- 有 `args.items` 数组 → `N items · M in progress`；
- **无** `items`（`todo_list` 无参、`todo_update` 仅 id/status）→ **空串**（att13；BDD `1928:1946:tests/bdd/steps_app_tui_transcript.rs`）。

**工具块 body** `humanize_tool_result_for_tui`：解析结果 JSON → `todo_list_body_lines`（glyph + content）；空表 → `(empty list)`（`269:282`）。

## 5. Prompt

三工具 **未** 覆盖 `prompt_guidelines` / `prompt_snippet`；继承 `TypedTool` 默认空（`46:48:src/infra/tools/typed.rs`）。`apply_tools_metadata` 仅聚合已选工具 guidelines（`140:145:src/agent/capabilities/tools_ops.rs`）。**无** 系统提示条文规定 rewrite vs update 选用时机（`src/agent/prompt/` 无 todo 关键词）。

工具面向 LLM 的语义目前主要靠 `description()` + JSON schema（`183:212`、`265:277:src/infra/tools/todo.rs`）。

## 6. 必须复用的测试接缝

### BDD 场景名

| Spec | Scenario | Binding |
|------|----------|---------|
| `agent-todo` | `todo-bar-default-shows-in-progress` | `tests/bdd/bindings_agent_todo.rs:6-10` |
| `agent-todo` | `todo-bar-resume-matches-live` | `bindings_agent_todo.rs:12-16` |
| `app-tui-transcript` | `todo-block-body-renders-checklist` | `bindings_app_tui_transcript.rs:137-141` |
| `app-tui-transcript` | `todo-summary-empty-not-placeholder` | `141-147` |
| `app-tui-transcript` | `todo-block-empty-list-body-hint` | `149-153` |
| `app-tui-fixed-zone` | `todo-bar-sits-between-queue-and-toast` | `bindings_app_tui_fixed_zone.rs:6-10` |

步骤实现：`tests/bdd/steps_app_tui_transcript.rs`（`1578+` todo 块 / 待办栏 / 空态）；`steps_app_tui_fixed_zone.rs`（`49-86`）；`steps_tools.rs`（`662-777` 十工具烟测含 todo_*，`939-961` 列举闭集）。

### 单元 / 集成单测

| 区域 | 测试名 / 文件 |
|------|----------------|
| 域 | `src/protocol/session/todo.rs` `mod tests`（`206:304`） |
| 工具+网关 | `src/infra/tools/todo.rs` `mod tests`（`313:521`）：持久化、`mutation_publishes_typed_todo_updated`、`dual_in_progress_rewrite_persists` 等 |
| 闭集/并发类 | `src/infra/tools/mod.rs` `builtin_concurrency_class_table`、`default_tools_include_todo_closed_set`（`121:174`） |
| 冻表 | `src/agent/capabilities/mod.rs` `freeze_includes_todo_builtins_first_turn`（`473:494`） |
| ReAct 流 | `src/agent/runtime/react/tests.rs` `todo_rewrite_publishes_todo_updated_into_run_stream`（`962+`） |
| compact | `src/agent/compaction/mod.rs` `compact_reappends_agent_todo_when_cut_drops_snapshot`（`1334+`） |
| LLM 前缀 | `src/agent/llm_project.rs` `agent_todo_custom_skipped_in_llm_prefix` |
| TUI bridge | `src/app/tui/bridge/tests.rs` `todo_block_humanized_and_checklist_rides_typed_event`；`session_tree.rs` `rebuild_todo_block_matches_live_flush`；`preview.rs` `todo_args_preview_is_item_count_not_json` 等 |

### SceneBuilder / HostSession 路径

- **SceneBuilder**：`src/app/tui/activity_fold/scene.rs`（`182:239`）；`todo_result` 先 `TodoUpdated` 再 `ToolExecutionEnd`。
- **BDD 直播 vs travel**：`rebuild_scrollback_from_travel` + `apply_xy_event`（`1659+`、`2008+:tests/bdd/steps_app_tui_transcript.rs`）；待办栏三步直接 `apply_xy_event(TodoUpdated)`（`1758:1786`）。
- **HostSession**：`src/app/tui/host/mod.rs`（`90`）；产品路径经 effects → `XyDriver` 事件流 → bridge，无 todo 专用分支；帧级断言多用 `InteractionBdd::from_model`（`1755`）。

## 7. 兼容性：旧 JSONL `agent_todo`

快照形态 `{ "items": [{ "id", "content", "status" }] }`（`77:78:src/protocol/session/todo.rs`）。`TodoItem` / `TodoList` **无** `#[serde(deny_unknown_fields)]`（`40:51`）— serde 默认**忽略**未知字段；多余顶层键在 struct 反序列化时亦被忽略。新增字段若要保持前向兼容，旧客户端仍只读三字段即可。

导出呈现：`session_export.rs` `[custom:agent_todo]` 块（`184`）。

## 8. `default_tools` 闭集（10 名）

`agent-tools.feature` r42 / r10：`read、bash、edit、write、grep、find、ls、todo_list、todo_rewrite、todo_update` 共 10（`17`、`73:llmanspec/specs/agent-tools/agent-tools.feature`）。`ask` 不计入（`177`）。实现 `default_tools_with_todo` 七文件工具 + `todo_tools` 三件套（`101:114:src/infra/tools/mod.rs`）。

**扩展策略**：新增第 11 个 builtin 名 → 破闭集 + 冻表单测 + BDD「10 个工具」步骤；**更稳妥**是在既有 `todo_*` 上扩 schema/端口方法，保持三名与 r1120 首轮冻表（`48:50:agent-todo.feature`）。

---

## If we change X, tests/specs that break

- **改 `TodoItem` 字段 / 校验 / 80 字上限** → `todo.rs` 域单测；r1118、r1842；`infra/tools/todo.rs` 持久化单测；BDD 清单 body / 待办栏。
- **改 mutation 返回 JSON（非全表 / 非 `{items}`）** → 三工具单测；`humanize_tool_result_for_tui` / `preview.rs` 单测；att36 BDD；r1125（成功返回全表）。
- **改 `TodoUpdated` 为 diff 或删字段** → `mutation_publishes_typed_typed_*`；`react/tests.rs` run stream；wire 往返单测 `event.rs`；r1122、r1718；待办栏 live 路径（`handlers/tools.rs`）。
- **引入「至多一条 in_progress」域拒绝** → `dual_in_progress_is_allowed`、`dual_in_progress_rewrite_persists`；r1126；`todo-bar-default-shows-in-progress`（多 doing 场景需扩）。
- **新增 todo 工具名（破 10 闭集）** → `default_tools_include_todo_closed_set`；`steps_tools.rs` `g_tools_all_ten` / `t_tools_ten_names`；`freeze_includes_todo_builtins_first_turn`；agent-tools r42/r10。
- **仅修 description 对齐 r1126** → 无行为测；若同时加校验则见上。
- **compact 不重挂快照** → `compact_reappends_agent_todo_when_cut_drops_snapshot`；r1119。
- **todo_list 开始发 TodoUpdated 或写 snapshot** → `mutation_publishes_*`（list 不发事件）；`todo_empty_fixture` 注释语义（`1949:1950:steps_app_tui_transcript.rs`）。
- **args_preview 对 todo_update 展示条目** → r1343 BDD `todo-summary-empty-not-placeholder`；`todo_args_preview_is_item_count_not_json`。
- **待办栏绘制 id / 新状态** → `todo_bar.rs` 单测；r1129 三区规则；`bindings_agent_todo` 两场景。
