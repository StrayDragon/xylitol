# HTTP 栈选型（六个里钉一个）

> 前提：**tagged JSON**；tokio；长期不做 Web；bang 直播是 attach 必做；**server 可进 Docker 当粗粒度沙盒**。
> 对照：[`http-json-tokio-top6.md`](./http-json-tokio-top6.md)、[`http-stack-revisit.md`](./http-stack-revisit.md)、[`topology-sandbox.md`](./topology-sandbox.md)、[`glossary.md`](./glossary.md)。

## 选定

**HTTP 栈 = axum 0.8。**

主路径：容器或本机进程听 **TCP** `127.0.0.1`（Docker 则 published port），WebSocket 上推 Event。本机 Linux 无 Docker 时 **可以**再挂 `UnixListener`，不是 Docker 沙盒的主路。

UDS/双听 API 上 **poem / salvo 更完整**（`combine` / `join`、bind 时 chmod）。对「TUI 在宿主机、server 在 Docker」这条主拓扑，那套 API 用不上。不换栈。论证见 revisit 篇，不是 Cargo 惯性。

## 其余否决（摘要）

| crate | 否决 |
|---|---|
| **warp** | 无一等 UDS。 |
| **actix-web** | worker/actor 税。 |
| **Rocket** | 整页重写；维护偏静。 |
| **poem / salvo** | UDS 赢家；换皮买不到 Docker attach，也买不到 `BashDelta`。 |

## 实现票仍要补的（与选 axum 无关）

1. **bang 直播**：产品 Event **`BashDelta`** + 已有 `BashResult`；Remote 不得只靠 REST。见 [`bang-and-tools-path.md`](./bang-and-tools-path.md)。
2. **WebSocket shutdown**：axum `on_upgrade` 会 `tokio::spawn`，停 host 要自己跟踪订阅者。
3. **Docker**：published port 为主；UDS 仅 Linux docker-ce 加分。见 [`docker-connect.md`](./docker-connect.md)。
