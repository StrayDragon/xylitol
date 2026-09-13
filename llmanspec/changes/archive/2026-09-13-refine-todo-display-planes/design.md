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

### D1 事件发射点：SSOT 变更点（工具执行 ctx 上行）——apply 期修订

原案（gateway 持有发布器 + composition root 接线 driver 事件流）在实施调查中
被推翻：host 下行只有一条路——driver run 流（`run_one` 的 `events` tx），gateway
的跨 run 通道要并入 per-run 流需引入 forwarder 生命周期管理；且 gateway 拿不到
执行上下文，反而**工具直接持有 `XyToolCtx`**。

修订案（实施）：`XyToolCtx` 增可选类型化上行 `state_events`
（`with_state_event_tx` / `publish_state`，镜像既有 `output_tx` 先例）；react
`run_one` 的 select 循环 drain 该通道并原样转发进 run 流。todo 三工具在 gateway
写成功后 `publish_state(TodoUpdated)`。性质不变：仍在 SSOT 变更点发布、无名字
魔串、无 JSON 解析、unbound 静默；且天然同构覆盖 in-process 与 host attach
（run 流 → `to_wire_event` → journal/mux）。

否决备选：runtime 按 `name == "todo_*"` 解析结果 JSON（host 内又一处字符串解析 +
名字匹配，与提案动机矛盾）。

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
