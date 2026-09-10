# Design: Driver / Command / dispatch 塌缩

> Designed / pre-start。硬依赖 `c2705`。本 change 是批次里最大的合约切片。

## 1. 目标

一条会话操作 = 一个 `Command` 变体 + **一个**执行函数。TUI slash、Print、HTTP unary、远程 Driver 都走它。`XyDriver` 不再复制该表。

## 2. 代码事实

| 路径 | 角色 |
|---|---|
| `src/protocol/wire/command.rs` | wire SSOT；含 `alias = "path"|"session_id"|"queue_stats"`（c2530 遗留） |
| `src/app/core/dispatch.rs` | Command → `XyDriver` 方法；自称纯 mapper |
| `src/app/core/driver/proto.rs` | ~50 方法的 trait |
| `src/app/core/driver/in_process/` | 本地实现 |
| `src/app/core/driver/remote.rs` | 每方法 `unary`/`unary_cmd` |
| `src/app/server/host.rs` `dispatch_session_unary` ~L1011 | 方法字符串 + payload；cog ~115 |
| `src/protocol/wire/registry.rs` | 方法名登记 |

今日循环：host 解 payload → 调 Driver 方法 → 与 dispatch 再 mirror。目标：host 解成 `Command` → `execute_command(runtime, cmd)` → JSON。

## 3. 分层后的 Driver

**保留在 trait（面/传输）：**

- `attach_session` / `refresh_surface_caches` / `drain_idle_events` / `link_health` / `take_resync_rebuild`
- `run` + `abort`（流式 turn）
- 只读固定区：`current_model` / `thinking_level` / `session_id` / `get_state` / `active_turn`（可由上次 Command 缓存，不必每人一个 RPC 方法）
- clipboard、project trust 等 TTY/本机（无 Command 变体则留面方法，或显式标 non-wire）

**移出 trait、改 Command 执行器：**

- select/cycle model、set thinking、bash、compact、export/import、fork/switch、messages/stats/tree、session CRUD、reload、resources、queue、steer/follow-up/clear（steer 若必须跟 run loop 绑定则 design 里标例外，与今日 dispatch 注释的 Prompt 同类）

**Prompt / Subscribe / ApproveTool / Quit：** 保持「非 dispatch」——见现有 dispatch 模块头。执行器不要假装覆盖它们。

## 4. serde alias 删除清单

一次性去掉（Pre-0.0.1，禁止双路径）：

- `ExportHtml`/`ExportJsonl`/`ImportJsonl`：`path` → 只 `output_path`/`input_path`
- `SwitchSession`：`session_id` → 只 `session_path`
- `GetQueueStats`：tag alias `queue_stats` → 只主 tag（与 registry 主方法名对齐后只留一个）

夹具、OpenAPI 调试文档、BDD 一起改。不要 `rename` + `alias` 并存。

## 5. host 复杂度

`dispatch_session_unary` 应变为：

1. 特殊路径：`Subscribe`、`Prompt`（流）、writer lease
2. 其余：`Command` serde 或 registry 已解析类型 → `execute`
3. 错误映射保持现有 `RpcResult` kind

禁止继续按 method 字符串手写 30+ 分支。

## 6. 远程 Driver

`XyRemoteDriver` 对「已是 Command 的操作」只 `unary(cmd)`。不要为每个 f 再包一层。

## 7. 验证

- 现有 dispatch / host / remote 单测平移到执行器。
- `just qa`；TUI harness 不改产品快捷键语义。
- `just complexity`：`dispatch_session_unary` 应明显低于现状 115（软信号，不作假硬闸）。
