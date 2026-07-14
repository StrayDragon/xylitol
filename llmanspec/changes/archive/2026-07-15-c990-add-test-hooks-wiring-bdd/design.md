# Design — c990-add-test-hooks-wiring-bdd

## 库优先（硬约束）

```text
嵌入方 / Print / TUI / Server
        ↓
embed：Driver + BuildAgentOptions.hooks_config
        ↓
agent（Option<XyHookBus>）+ provider adapters
        ↓
XyHookBus（精选导出）← 可替换
```

- **Subject**：`InProcessDriver` / `build_agent` / agent 已接线 API
- **不是**：`HostSession`、bang UI、picker UI（那些只是调用同一 Driver）

`XyEvent` = 面向 client 的流；`XyHookBus` = 面向扩展/脚本的库端口。二者并列。

## 精选导出

| 符号 | 理由 |
|------|------|
| `XyHookBus` | 可替换端口，与 `XyModel` 同级 |
| `XyHookOutcome` | 端口结果类型 |
| `NoopHookBus` | 对称默认 / 测试 |

不导出 `HookDispatcher` / `HookEvent`（infra 细节；脚本配置仍走 `hooks_config`）。

## 与 `hooks.feature` 分工

| 文件 | 证明 |
|------|------|
| `hooks.feature` | 调度器机制 |
| `hooks-wiring.feature` | 库 API 被调用后 bus 真收到事件 |

## 操作字典（库 API）

| 操作名 | 库路径 | 预期事件 | 启用 |
|--------|--------|----------|------|
| `确保新会话` | Driver→`ensure_session`（经 `session_tree` 等） | `session_start` | **c990 smoke** |
| `跑一轮 agent` | `Driver::run` | `agent_start` | 可选 |
| `选择模型 fake` | `Driver::select_model` | `model_select` | c996 |
| `设置思考级别 high` | `Driver::set_thinking_level` | `thinking_level_select` | c996 |
| `切换会话 target` | `Driver::switch_session` | `session_before_switch` | c995 |
| `打开会话树` | `Driver::session_tree` | `session_before_tree` | c995 |
| `执行 bash` | `Driver::execute_bash` | `user_bash` | c997 |
| `Completions 发送流式请求` | Completions adapter | HTTP 三缝 | c998 |

未登记操作名 → 步骤失败，消息含操作名。

## Smoke 策略

1. `hooks_config` / `HookDispatcher` 作 `XyHookBus` 注入 agent（与 composition 同构），或测试用录制 bus（实现同一 trait）。
2. 孤儿 `session_id` + `Driver::session_tree` → `ensure_session` 新建 → `session_start`。
3. 禁止测试内直接 `dispatcher.dispatch(...)` 当作 wiring 通过。

## 预留

c996/c998 行仅 design / feature 注释；不绑定失败 `#[scenario]`。
