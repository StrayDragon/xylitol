# 设计：ReAct 运行期补发 AgentStart 事件

## 1. 问题定位

`XyEvent::AgentStart { session_id, model }` 已有完整下游：`xylitol-wire` 双向映射、`xylitol-protocol::Event::AgentStart`、TUI bridge 置 `Working`、`XyRemoteDriver::is_cold_replay_tape` 冷回放过滤、Print 忽略分支、harness 默认脚本。唯独 `xylitol-runtime` 没有任何发射点（`grep -rn "AgentStart" crates/xylitol-runtime/src` 仅命中 `thought_clock` 的内部枚举，与事件无关）。

后果是事件流缺一个可对齐的 run 起点：TUI 只能靠 `TurnStart` 推断，冷回放磁带过滤在真实链路上永不触发（该分支只在单测注入下走到）。

## 2. 发射点

`run_react_loop` 的 `async_stream` 内，在下列条件全部满足后发一次：

1. `agent.turn` 观测 span 已建立（`AgentTurnSpan::start_with_session` 返回）；
2. `history` 已装配完毕（session env、session prepare、本轮用户消息均已落盘）；
3. 首个 `TurnStart` 之前。

即插在 `agent_turn_span` / `turn_obs_parent` 构造之后、`'outer: loop` 之前。该位置在两条入口（`start_root_stream` 与 `pending_root_stream`）共用的 `build_live_react_stream` 下游，因此排队准入路径同样覆盖。

**为什么不在 `submit_parts` 发**：那里还没装配 history，且 `Admit::Pending` 要等 lease 拿到 token 才算真正开跑；在流内发才能保证「发出即代表本 run 已实际启动」。

**为什么不复用脚本 hook `agent_start` 的位置**：那是 hook 观测，不产事件，且在 history 装配前；两者语义不同，顺序上 hook 先、事件后没有冲突。

## 3. 字段口径

| 字段 | 取值 | 理由 |
|---|---|---|
| `session_id` | 本 run 绑定的 session id（与 `agent.turn` span 的 `ObsSessionContext.session_id` 同源） | 供 trace 对齐 |
| `model` | 本 run 启动时 `ModelManager::current_model().id`，与已算出的 `model_api` 同一把锁、同一时点 | 「启动快照」语义；流内不重读，避免中途 `select_model` 让首事件与 span 属性不一致 |

`current_model()` 为 `None`（未选模型）时取空串：该路径本就会在首次 generate 处失败，`AgentStart` 仍应作为首个事件出现，便于端侧把失败归到具体 run。

## 4. 顺序契约

- `AgentStart` 是本 run 事件流的**第一个** `yield`；
- `AgentEnd` 仍是**最后一个**；
- 二者之间允许任意 `TurnStart/TurnEnd/Message*/Tool*`。

既有「`MessageUpdate` 含工具意图早于 `ToolExecutionStart`」「`ToolExecutionStart` 不早于 `MessageEnd`」等顺序规则不受影响（它们约束的是消息与工具段，不是流首）。

## 5. 失败与中止

- `ensure_session` / `load_history` 失败时流只有 `Error`，不发 `AgentStart`（run 未真正开始）；
- 取消中止：`AgentStart` 已发，随后 `XyEvent::aborted()` + `AgentEnd`，与既有中止语义一致；
- 中间件故障结束 run：`AgentStart` 已发，随后 `Error` + `AgentEnd`。

## 6. 兼容性

- wire / protocol / bridge / Print 均无需改动（映射与消费分支已存在）；
- 冷回放磁带过滤（`is_cold_replay_tape`）从「仅单测可达」变为真实链路可达，属于修复而非行为回退；
- `AgentEnd.messages` 语义不变。

## 7. 验收

- `agent-runtime` 新增一条规则 + 两个可执行场景（首/末事件、字段口径），由 `tests/bdd` 绑定执行；
- 全量 `just qa` 绿，且既有 1029 个 BDD 场景不回归。
