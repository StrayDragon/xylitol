# 启动就绪窗口 外部对照笔记（c2465）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**启动窗口保护**（其 server 进程模块 `process.ts:156-177`）：

- 端口先绑定、应用未装配完成期间，dispatch 层单独拦截应答：
  - `/api/health`：按内部状态回 200 / 500 / 503；
  - 其余请求：503 + 语义化错误码 `service_starting` / `service_stopping` / `service_failed`
    + `retry-after: 1`。
- 客户端因此能区分三种局面：稍等重试（starting）、别等了（failed）、正在下线（stopping）。

**状态机消费侧**（CLI 服务状态与客户端连接层）：

- ensure() 探活把 503-starting 视为「还没好，继续等」，把连接失败与启动失败分开处理，
  重试策略据此分支（指数退避 vs 快速失败）。

## 设计动机

- 端口绑定 ≠ 服务可用。把「绑定了但在装配」显式化，避免客户端把启动窗口误判为死服务
  而提前放弃或误杀重启。
- `retry-after` 让退避策略由服务端主导，客户端不用猜间隔。

## xylitol 现状核对（2026-08-23）

- `src/app/server/oapi.rs:25`：`/healthz` 已在 OpenAPI 调试文档注册，但语义简单
  （存活即 200），无 starting/failed 区分。
- `src/app/server/runtime.rs`：先 build `HostState` 再 bind serve；绑定后到路由就绪之间
  的窗口行为未显式定义（trust / MCP / provider 装配都在这条路径上）。
- 产品 TUI attach 固定端口，窗口期请求只会得到连接层笼统失败。

## 落点判断

- 改动集中在 runtime/http 装配顺序：bind 后先挂最小应答器（503+retry-after），
  装配完成后切换完整路由。小改、无产品模型变化。
- `/openapi.json` 与 BDD 场景随动（见 proposal Impact）。

## 相关但本票不取

- 健康检查鉴权（参考实现把 health 也放在凭据门禁后防匿名探测）——归 c2485 拍板。
