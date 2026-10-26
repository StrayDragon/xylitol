# ReAct 内环中间件设计

## 1. 为什么是「中间件」而不是「可换 loop」

| 方案 | 单写者 / 取消 / 权限终裁 | 插件可参与 | 结论 |
|---|---|---|---|
| 插件 `implements AgentLoop` 整段替换 | 丢（每插件自带调度） | 全量 | **禁止** |
| 纯旁路 hooks（现状） | 保 | 只能观察 / 阻断 | 已存在，保留 |
| **有序中间件链 + core 调度** | 保（core 持 run 句柄） | 七阶段有界参与 | 本 change |

中间件拿到的是**借用视图 + 返回值**，不持有 run 控制权：不能自己发事件、不能自己调模型、不能绕过 core 落盘。

## 2. SPI 形状

```text
阶段闭集（LoopStage）: before_turn | before_model | after_model
                        before_tool | after_tool
                        decide_continue | after_turn
控制闭集（LoopControl）: continue | stop | compact | handoff
返回值契约            : { ctx, control }
```

- 每阶段一个独立端口（`dyn` + `async_trait`），注册表按 `(stage, order, id)` 有序；同 order 按注册序。
- 注册表是 `Ctx` 服务（`xylitol-plugin-sdk` 提供键与类型）；`register(...)` 返回卸载句柄，插件把句柄放进 `Effect::Disposer`，`stop` 时卸干净。
- 链在 **run 启动时快照**（`freeze_root_config`），与既有「mutable-slots-next-turn」语义一致：进行中 run 不受后挂中间件影响。
- 链为空 → 不建 span、不 clone、不 await，直接走基线路径。

## 3. 阶段语义与执行点

| 阶段 | core 位置 | 可改 | control | core 之后做什么 |
|---|---|---|---|---|
| `before_turn` | `TurnStart` 之前 | 系统碎片 | `continue` / `stop` | `stop` → 本 run 直接 `AgentEnd`，不发 `TurnStart` |
| `before_model` | skills 展开之后、`project_outbound` 之前 | messages / tool schema / model 参数（thinking、max tokens、system） | `continue` / `stop` | `stop` → 本 run 结束；core 才发起 `generate_stream` |
| `after_model` | 流结束、持久化之前 | assistant parts / tool_calls | `continue` / `stop` | 改写后由 core 持久化并按 tool_calls 规划工具批 |
| `before_tool` | `ToolExecutionStart` 之后、执行之前 | args / 路由（工具名） | `continue` / `stop` | **core permission 用新 args 重裁**；`stop` → 本 run 结束 |
| `after_tool` | 工具结果生成后、写 history 之前 | 结果文本 / `is_error` | `continue` | 截断/摘要后的结果进 history 与事件 |
| `decide_continue` | Settle 的 `TurnEnd` → settlement → compact 预检 → `should_stop_after_turn` **之后** | 无（只看 ctx） | `continue` / `stop` / `compact` / `handoff` | 见 §4 |
| `after_turn` | 每次 iteration 的 `TurnEnd` 之后 | 只读 + 可选 ctx 附加 | 仅 `continue` | 其他取值记 trace 并忽略（不得静默改行为） |

`ContinueTools`（仍有 tool_calls）**不**调用 `decide_continue` 与 `should_stop_after_turn`，与 r1843 / r1041 一致；`after_turn` 两种 iteration 都调用。

### 为什么 `before_model` 在 skills 展开之后

Skills 展开今天由 core 直接做（`expand_skills_in_agent_messages`）。把它挪成官方 middleware 会改变默认路径的装配顺序与失败面（读文件失败等），超出本 change 的「无 middleware 行为不变」硬要求。因此本 change 保留 core 展开，并把 `before_model` 放在展开之后——后挂插件看到的是展开后的 messages，语义上等价于「skills 是链上的第一环」。迁移成官方 middleware 记入后续方向，不在本 change 落空壳。

## 4. `decide_continue` 语义

- 只在 **Settle**（本轮不再续跑工具）调用一次；`continue` → core 继续 outer loop（下一轮 generate）；`stop` → `AgentEnd`，**不得** drain steer / follow_up；`compact` → core 执行一次强制 compaction，然后继续；`handoff` → 本 change **保留枚举位但不实现**：core 以稳定错误 `LoopMiddlewareHandoffUnsupported` 结束本 run（可观测，不静默）。
- **不得**插件自行再调模型：枚举是唯一出口，下一次 `generate_stream` 只能由 core 调度。
- **硬上限**：core 统计「连续 `continue`/`compact` 且本轮无新工具结果、无新用户消息」的次数；超过上限（常量，32）→ 停止本 run 并发一次 `XyEvent::Error`（kind `LoopMiddlewareLimit`），随后 `AgentEnd`。选 Error 而非静默停：插件 bug 必须可观测；Print 面因此非零退出符合既有 `cli-print` 契约。
- 与 `should_stop_after_turn` 的顺序：hook 先（既有硬停语义一字不改），middleware 后。无中间件时行为与基线逐事件一致。

## 5. 权限「只能更严」

```text
脚本 hooks（block/modify）
  → AgentHooks.before_tool_call（deny）
  → middleware.before_tool（改参 / deny / 改路由）
  → core permission（用最终 args 重新计算 target 后裁决）
  → 执行
```

- middleware 的 deny 只会让结果更严；**放行不撤销**上游 deny，也不绕过 core permission。
- 改参后 core 必然重算权限 target（路径 / 域名），因此「改参绕过权限」不可达。
- 需要「更松」的插件必须显式 capability + 用户信任，属未来 change，不在本 change 开口子。

## 6. 取消与超时（core 永远拥有取消）

每次中间件调用都在 `tokio::select!` 里与 `cancel.cancelled()`、单次超时（常量，2s）竞速：

- 取消 → 本 run 以 aborted 结束（与现有 abort 语义一致）。
- 超时 → 记 `loop.middleware` error 事件属性 + 停止本 run（kind `LoopMiddlewareTimeout`），不得挂死 turn。

## 7. 可观测

`loop.middleware` span（父 = 当前 iteration span），属性：

| 属性 | 内容 |
|---|---|
| `loop.middleware.id` | 注册 id |
| `loop.middleware.stage` | 阶段名 |
| `loop.middleware.order` | 排序值 |
| `loop.middleware.control` | 返回的 control |
| `loop.middleware.changed` | 改动字段摘要（`messages` / `tool_schemas` / `model` / `assistant` / `args` / `result` / `system_fragments`） |
| `loop.middleware.outcome` | `ok` / `error` / `timeout` |

仅在 `provider_trace_active()` 时创建（与既有 obs 门一致）；链空时不建 span。

## 8. 组合根与官方示例插件

- 核心插件 `agent-react`（名字不变，`CORE_PLUGIN_NAMES` 不扩）`ctx.provide(LoopMiddlewareRegistry)`。
- `Kernel::boot` 取该服务 → `driver.set_loop_middleware(registry)` → `AgentRuntime`；与 `set_prepare_hub` 同型。
- 新 crate `xylitol-official-plugin-loop-example`（config-gated）：
  - `before_tool`：对 `bash` 的危险模式（如 `rm -rf /`）deny —— 只更严。
  - `decide_continue`：settle 计数达上限后 `stop`（自定义策略示例）。
  - 配置段 `loop_example.enabled`（默认 false）；未配置 → `default_rows` 不插行 → 零装配、零成本。
- 插件之间不互相 `use`：config 只 re-export 示例插件配置类型（沿用 config→hooks / config→mcp 的 peer 例外模式），示例插件只依赖 `plugin-sdk`。

## 9. Hooks 共存

- `XyHookBus`（脚本 JSON）与 `AgentHooks` 原样保留：审计 / 遥测 / 兼容一期。
- 不把 hooks 改成 middleware，也不做双写；两者在工具边界的先后见 §5。
- 迁移建议写入后续 change，不在本 change 删除任何 hook 语义。

## 10. Strategy 槽（不做）

用户给的「同一时间仅一个 `ReActStrategy`」是方向。本 change **不落 trait / 不落注册点**（AGENTS：不得为未交付方向预埋空壳 `Xy*`）。今日行为 = 默认策略；`handoff` 枚举位为将来 Strategy 切换留出口。

## 11. 风险与取舍

| 风险 | 处置 |
|---|---|
| 中间件拖慢热路径 | 链空零开销；每步 span 仅在 trace 激活时建 |
| 插件死循环 / 挂死 | `decide_continue` 硬上限 + 单次超时 + 取消竞速 |
| 插件绕过权限 | 权限终裁在 core，改参后重算 target；「只能更严」有 BDD 场景 |
| 与 hooks 语义冲突 | 明确先后顺序；hooks 不改语义 |
| 快照与热挂不一致 | 与既有「下一轮生效」语义一致，文档写明 |
| 示例插件变成隐性产品负担 | config-gated、默认不装、零配置零成本有测试 |
