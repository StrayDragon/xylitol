# Design — refine-todo-display-planes

## 信息平面基线（本 change 的分类法）

| 平面 | 真值源 | todo 的消费方式 | 本 change 后 |
|---|---|---|---|
| LLM API body | `project_for_llm`（session → LlmMessage） | 工具调用 args + 工具结果 JSON 文本 | **不变**（模型接口） |
| 会话 SSOT | `SessionEntry` 树 + `agent_todo` Custom latest-wins | 工具写入 / atd10 压缩保快照 | **不变** |
| 恢复投影 | `rebuild_scrollback_from_travel` ← SSOT 快照 | 类型化，已干净 | 不变（仅 body 渲染换 helper） |
| client 展示（live） | `apply_xy_event` ← XyEvent | **解析 `result: &str` JSON** | **改：消费类型化事件** |
| 日志 | provider-trace JSONL + level log | 观测面 | 不变 |

## 决策

### D1 事件发射点：SSOT 变更点（gateway），非 runtime 名称匹配

三个候选：

1. `XyToolCtx` 加通用事件上行 → 拓宽 tool port，只有一个工具族用，否。
2. `tool_exec.rs` 在 End 后按 `name == "todo_*"` 匹配并解析 result JSON → host 内
   又出现一处字符串解析 + 名字魔串，与提案动机矛盾，否。
3. **`SessionAgentTodoGateway` 持有可选事件发布器**（`Option<UnboundedSender<XyEvent>>`
   或等价回调），三工具成功 persist 后发布 `TodoUpdated`；composition root（app 组装
   处）接线到 driver 事件流。选此。

理由：SSOT 变更即语义事件（change-stream 语义），无名字匹配、无 JSON 解析；
层级合法（infra → protocol 依赖允许）。unbound（无发布器）时静默跳过——单测 /
Print 面零负担。

### D2 事件形状

`XyEvent::TodoUpdated { list: TodoList }`：直接复用 `protocol::session::TodoList`，
bridge 侧 `sync_todo_checklist(model, list)` 原样可吃。空表 = 清除 checklist 行
（既有语义）。serde camelCase tag 与枚举一致。

### D3 冷回放归类：非 tape

`TodoUpdated` **不进** `is_cold_replay_tape`：resume 的 checklist 由 SSOT 快照重建
（atd9），事件是状态推送而非叙事 tape；回放重画虽幂等，语义上应避免第二来源。

### D4 呈现字形与摘要收口在 client 展示层

- `[ ]/[~]/[x]/[-]` 字形表现散在 `session_tree.rs` 一处；todo_* 块 body 渲染加入后
  收口为单一 helper（TUI 内部展示词汇，不进 protocol）。
- args 摘要：条目计数式（如 `5 items · 1 in progress`），不展开 items JSON——
  与 att54「摘要 MUST NOT 携带完整 args JSON」同族。
- `output_looks_like_machine_json` 补 `{"items":[…]}` 形态识别（安全网，兜 orphan）。

### D5 线协议与降级

`protocol::Event` 映射 `TodoUpdated`（远程 attach 不丢刷新）；未升级端按 pa-wire1
既有规则降级忽略，不 panic。老 host + 新端 = 收不到事件、checklist 仅 resume 时
刷新——可接受降级，不双发兜底。

## 风险

- `XyEvent` 是 wire 闭集：新增变体波及 `remote.rs` 冷回放表、protocol::Event 映射、
  以及各 `match` 穷举点（编译器兜底）。
- checklist 行与 todo_* 块 body 信息冗余（块折叠时不可见，展开才见）；att24 已裁决
  Used 保留时间线，冗余是决策内代价。

## 测试 seam（复用既有，不发明新 seam）

- 协议 roundtrip / 降级：protocol 既有单测。
- gateway 发射：`src/infra/tools/todo.rs` 既有测试基座（tempdir store + bind_session）。
- bridge live / rebuild 幂等：`src/app/tui/bridge/tests.rs` + `session_tree.rs` 既有
  测试；transcript BDD 经 `tests/features` 既有 step（rstest-bdd，`cargo test --lib
  --all-features tests::bdd::`）；insta 快照走 test-tui-harness 既有 SOP。
