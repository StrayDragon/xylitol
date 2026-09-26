# Design: JSON-RPC 信封映射

内部 IR 仍是 `RpcMessage` 四象限枚举。只换**字节**。双读窗口内 codec 按首字节对象的 `jsonrpc` vs `type` 选方言；应答跟请求方言。

## 字段

| IR | JSON-RPC 2.0 |
|---|---|
| `rpcId` | `id`（string） |
| `method` | `method` |
| `payload` | `params.payload` |
| `writerToken` | `params.writerToken`（缺省省略） |
| `ServerResponse` 成功 | `result` = 今日 `RpcResult.value`（不再包 `{ok:true}`） |
| `ServerResponse` 失败 | `error: { code: -32000, message: details, data: { code } }` |
| `ServerHello` | notification：无 `id`，`method: "server/hello"`，`params.protocol` |
| mux `ServerRequest` | 带 `id` 的 request（应答仍走 POST `/api/respond`，**不**在 WS 上回） |
| `ClientResponse` | `{"jsonrpc":"2.0","id","result": payload}` |

`-32000` 只表示「应用错误载体」，产品码永远是 `data.code` 字符串。非法信封仍 HTTP 4xx，body 不含业务成功形态。

## 为何不把审批改到 WS 上行

`protocol-app` 禁止全双工 WS 外层。JSON-RPC peer 的自然形状是单管道；本产品保持拆开通道，因此这是「JSON-RPC **形状的字节** + 既有拓扑」，不是标准单流 JSON-RPC。通用库能 parse unary；反向 RPC 仍要手写 respond POST。

## 版本

双读期间 `PROTOCOL_VERSION` 不 bump。翻 `HttpWsClient` 只发 JSON-RPC 时 bump；旧客户端死于 hello mismatch。
