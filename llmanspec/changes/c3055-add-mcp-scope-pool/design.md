# Design — MCP scope pool 与连接生命周期

## 1. Scope 与 connection key

- `McpScope`：`Shared` / `Workspace` / `Session`；缺省 `Workspace`。
- `McpServerConfig` 增 `scope` 与可选 `credential`。
- connection key = `(config fingerprint, scope, credential, workspace, session)`：
  - `fingerprint` = transport / command / args / url / headers / env 的稳定哈希；同名不同 config 必须独立连接。
  - `Shared`：workspace / session 为空，全进程共用。
  - `Workspace`：每个 workspace 一个 key。
  - `Session`：每个 session 一个 key（预留，按 key 隔离）。
  - `credential` 为空时表示无凭据；c3065 把 credential id 放进 key。
- 同 key 共用一个 `McpClientManager`。

## 2. Pool 与引用计数

- `McpConnectionPool`：key → `PoolEntry { manager, servers, claims, last_connected, last_diags }`。
- 会话 claim：每个 session 对其 workspace set 的每个 workspace 建 claim；1:N 时持多个。
- 懒连接：首次 claim 才连接；未 claim 的 server 不 spawn。
- 释放：最后一个 claim 释放才 shutdown 并从 pool 移除。
- workspace set 人类变更触发 claim 增删；LLM 不得触发。

## 3. 生命周期

- Switch / Resume：**不 reset**；保留现有 pool 与连接。
- Reload：清空 pool（shutdown 全部），下次 ensure 重连。
- 连接失败按 server 重连；单 server 失败不拖死其余。
- 保留上次已知快照：pool 忙 / 正在重连时 `view` 返回上一份 `SessionPrepareView`，不得把未知当 configured=0。

## 4. Workspace 路由

- stdio 连接设 `current_dir(workspace)`。
- Streamable HTTP 每 scope 一条 transport。
- tool adapter 持所属 pool entry 的 manager；`XyToolCtx.workspace` 不在 session set 内由 runtime 拒绝（c3050）。
- session scope 需要 session id；本 change 用 key 的 session 字段预留。

## 5. SessionPrepare 集成

- `SessionPrepare` 事件增 `workspaces: Vec<String>`（primary 在前）。
- hub 按事件 workspace set 建 claim / 连接；无 workspace 时回退 agent cwd。
- `SessionPrepare` 的 `fingerprint` / `env_body` 按当前 claim 的 tool 集聚合。
- 连接过程仍经只读缝暴露 progress / snapshot。

## 6. 测试

- key：fingerprint / scope / credential 隔离；同名不同 config 独立。
- pool：两 session 同 workspace 共用 manager；释放一个仍保留；全部释放后 shutdown。
- 生命周期：switch / resume 后 pool 不变；reload 后清空。
- 快照：pool 忙时 view 保留上一份。
- stdio `current_dir`（fixture 记录 cwd）。
