# Loop 可扩展性 as-is 调研（c2944 explore 纪要）

只读探查，无应用代码改动。事实来源：`crates/xylitol-runtime/src/agent/runtime/react/mod.rs`、
`runtime/hooks.rs`、`runtime/tool_exec.rs`、`crates/xylitol-plugin-sdk/src/ports/hook.rs`、
`plugins/xylitol-official-plugin-hooks/src/plugin.rs`、`crates/xylitol-kernel/src/boot.rs`。

## 1. 今日 loop 调用图

```
XyDriver::run
 └─ AgentRuntime::submit_root / submit_parts
     ├─ freeze_root_config  (tools / hooks / permission / hook_bus / skills / cwd / compaction 快照)
     └─ build_live_react_stream
         └─ run_react_loop
             ├─ agent_start 脚本 hook
             ├─ 'outer loop
             │   ├─ TurnStart
             │   ├─ drain steer → 落盘 user 消息
             │   ├─ messages = history.clone() + expand_skills_in_agent_messages
             │   ├─ context(pre) 脚本 hook（可 block）
             │   ├─ prepare_turn_binding（每 turn 重读 model / thinking）
             │   ├─ project_outbound(messages) → attempt_model_stream(model, …)
             │   ├─ 流式 chunk → MessageStart/Update/End
             │   ├─ persist assistant
             │   ├─ continue_after_tools = !tool_calls.is_empty()
             │   ├─ 无 tool → close_iteration(Settle)
             │   │    └─ TurnEnd → settlement → overflow/threshold compact
             │   │       → should_stop_after_turn → drain steer
             │   └─ 有 tool → tool_batch plan → tool_exec::run_one ×N
             │        └─ close_iteration(ContinueTools)（TurnEnd，不 settlement/不 compact/不 should_stop）
             └─ follow_up drain → agent_settled / agent_end → AgentEnd
```

- **唯一 model 调用者**：core `run_react_loop`。
- **唯一 tool 调用者**：core `tool_exec::run_one`。
- **续跑判定**：`!tool_calls.is_empty()` + `should_stop_after_turn` 单槽 + steer/follow-up 队列，全部 core 内写死。

## 2. 现有扩展点与缝

| 扩展点 | 定义处 | 能力 | 缺口 |
|---|---|---|---|
| `AgentHooks` | `runtime/hooks.rs` | `before_tool_call` 仅 deny；`after_tool_call` 改结果；`should_stop_after_turn` 单槽 | 单进程内装配，非 rutis 生命周期；无 `before_model` / `after_model` / `decide_continue` |
| `XyHookBus` | `plugin-sdk/ports/hook.rs` | 脚本 JSON 旁路：`tool_call` pre/post、`context`、`turn_*`、`message_*`、`model_select` 等；block/modify | 旁路语义；不能改 messages / tools schema / model 参数；无续跑裁决 |
| rutis 插件事件 | `plugin-sdk/plugin_events.rs` | `ToolPreExecute` / `ToolPostExecute` / `AgentPreStep` 已声明、hooks 插件已 `on(...)` | **runtime 从未 emit 这三类事件**（实际只 emit `SessionPrepare`），现状是空接缝 |
| `Ctx` 服务 | `plugin_keys.rs` | `session_store_key` / `hook_bus_key` / `permission_key` / `tool_vec_key` / `session_prepare_hub_key` | 无 loop 决策服务 |
| 会话准备瀑布 | `session/prepare` | MCP 首轮门、工具表冻结 | 只影响工具表，不影响 loop 决策 |

**结论**：插件今天最多「观察 + 阻断 + 改工具参数/结果」，无法参与 turn 边界、模型请求整形、assistant 改写、续跑裁决。

## 3. 与三柱对齐现状

- **Skills**：`expand_skills_in_agent_messages` 在 core 内直接调用（`before_model` 等价点）。本 change 保留 core 默认路径；「迁成官方 middleware」记 design 权衡，不在本 change 强制。
- **MCP**：只注册 `XyTool`；工具结果摘要今天由 `after_tool_call` / `tool_result_post` 旁路做。目标形态是 `after_tool` 中间件，但**不进 MCP bridge**。
- **rutis 插件**：官方插件只依赖 `plugin-sdk`；插件之间不互相 `use`（`check_crate_graph.py` 的 `PLUGIN_PEER_EXCEPTIONS` 只允许 config→hooks / config→mcp / obs→config）。

## 4. seam 候选（已选定）

| 面 | 选择 | 理由 |
|---|---|---|
| SPI 落点 | `xylitol-plugin-sdk/src/ports/loop_middleware.rs` + `loop_middleware.rs` 词表 | 插件作者只依赖 sdk；端口以 `dyn` 注入，保持 `async_trait` |
| 注册表提供者 | kernel 核心插件 `agent-react`（`ctx.provide(LoopMiddlewareRegistry)`） | 不新增核心插件名；`CORE_PLUGIN_NAMES` 不变 |
| 传入 runtime | `Kernel::boot` 取 `Ctx` 服务 → `driver.set_loop_middleware(registry)` → `AgentRuntime` | 与既有 `set_prepare_hub` 同型，不新造装配路径 |
| 快照时机 | `freeze_root_config` 取有序链快照 | 与「mutable-slots-next-turn」既有语义一致：进行中 run 用启动快照 |
| 零开销 | 链空时直接跳过，不建 span、不 clone | 保住「无 middleware = 基线」 |
| 观测 | fastrace `loop.middleware` span + 属性 | 复用 `provider_trace_active()` 门与 `SpanCollectScope` 测试缝 |
| 测试 seam | 既有 BDD harness（`fixtures.rs::AgentState` + `helpers.rs::make_agent`）+ kernel 契约测试 | 不另造 harness |
| 示例插件 | 新 config-gated crate `xylitol-official-plugin-loop-example` | 证明第三方可在 rutis 生命周期内注册/卸载；默认 rows 不装 |

## 5. 关键约束复核

- **单写者 / 取消 / 权限最终裁决** 留在 core：中间件拿到的是借用视图 + 返回值，不能持有 run 控制权。
- **权限只能更严**：`before_tool` 的 deny 在 core permission 之前生效；放行不绕过 core permission。
- **`decide_continue` 不得私自调 model**：枚举硬契约，core 调度下一次 generate。
- **不得新增空壳**：Strategy 槽本 change 不落 trait（AGENTS「不得为未交付市场预埋空壳 `Xy*`」）。
- **版本命名闸**：新标识符不带 `v2` / `_v3` 段。
- **uncomment 闸**：改动行不新增可移除注释；追溯写 specs / 提交信息。
