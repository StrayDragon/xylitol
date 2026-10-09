# 设计：fory-only 产品 /rpc

## 1. 决策表

| # | 决策 | 理由 |
|---|---|---|
| D1 | 公开 `/rpc` 只接受 v3 二进制；JSON 文本与非 fory 字节 = 非法信封（HTTP 400） | 产品客户端已 v3；双读是调试遗留 |
| D2 | `GET /healthz` 留在同一监听器 | attach 探活用同一 origin；标准库没有 HTTP；另起端口才是多余服务 |
| D3 | 删除 `/openapi.json`、`/docs` 与 OpenAPI 依赖 | 文档描述 JSON 信封，fory 化后无真源价值 |
| D4 | 内部仍可将 fory 解开后喂既有分发 | 内核重写不在本 change；产品不可观察 |
| D5 | 不换 HTTP 框架；只关未用默认 feature、去掉 OpenAPI crate | 业务只用两文件；换框架另开 |
| D6 | 双轨 BDD **少删**：保留行为覆盖，GWT 改产品语义；场景 id 去 jsonrpc/v3 前缀；JSON 仍可服务改为拒绝 | 保留覆盖后收口词表 |
| D7 | `host.describe.formats` 只含 `fory-v3`；attach 只说 v3，不降级 | 与 r1911 硬闸同向，去掉 jsonrpc 标识 |
| D8 | 不 bump `PROTOCOL_VERSION`（保持 3）；无 `migrations/` | 个人工具同步发版；旧客户端失败闭合 |
| D9 | 写者租约：HTTP 仍 `X-Writer-Token`；WS 仍连接本地租约 / 应答 `writerToken` 语义，载体改为 v3 帧字段 | 产品语义不变，钉帧类型的条款改载体无关 |

## 2. 公开面 vs 内部方言

```
客户端（TUI/CLI）          Host 监听器                 分发
  fory v3 帧    →    POST/WS /rpc 只认 fory    →  (内部) 再入既有方法表
  JSON 文本     →    400 非法信封              ×
  GET /healthz  →    同端口探活
  /openapi /docs →   不存在
```

## 3. spec 收口映射

| @req | 终态 |
|---|---|
| server-core r1778 / r1796 | 产品入口 POST/WS `/rpc`，**仅** v3 二进制；MUST 暴露 `/healthz`；MUST NOT 暴露 JSON 文本 `/rpc`、`/openapi.json`、`/docs` |
| server-core r1784 | 改为 MUST NOT 提供 OpenAPI / Scalar |
| server-core r1780 / r1786 / r1787 | healthz 与就绪窗口 **不变** |
| server-core r1803 | 下行只走 v3 `ServerNotification`；method 名集合不变 |
| server-core r1804 | WS 只接受 v3 binary（+ ping/pong/close）；JSON 文本上行 MUST 拒 |
| server-core r1902 | v3 为唯一产品载体；删「调试通道 MUST 保活」 |
| server-core r1908 | 对拍改为 fory-only 回归锁定原行为（事件/快照/树）；MUST NOT 再要求 JSON 孪生 |
| server-core r1909 | 硬切完成 = JSON 文本通道移除；MUST NOT 第三载体 |
| server-core r1911 | formats 至少且产品默认仅 `fory-v3` |
| server-core r1921 / r1922 | 去掉「与 JSON 轨对拍」；保留 fory 侧 RAW / 深树语义 |
| server-core r1928 | 非 fory `/rpc` body（含 JSON 对象与数组）一律非法信封 |
| protocol-app r1701 / r1709 | 单一产品载体 = v3；JSON 样例解析可留作内部方言单测，不作为 HTTP 产品通道 |
| layer-architecture r1532 | server 面经 v3 `/rpc` 暴露，不再写 JSON-RPC 2.0 为产品真源 |
| app-tui-bridge r1182 | 默认 attach 走 v3 客户端；POST `/rpc` 仍在，载体为 fory |

## 4. BDD 少删

保留并改跑产品路径：`product-path-event-equivalence`、`product-path-session-snapshot`、`session-tree-raw-carrier`、`session-tree-deep-restore`、租约/幂等/审批/订阅等 `steps_server` 产品场景。

改为拒绝断言：`json-text-rpc-rejected`、`openapi-debug-doc`、`ws-text-unary-rejected` / `ws-text-unary-rejected-peer`、JSON batch。

`cutover-requires-parity-green`：改为可观察「产品客户端默认 v3 且 JSON `/rpc` 被拒」，不再读源码字符串。

## 5. 退路

发现 fory 映射缺口导致产品 unary 失败：该缺口必须先修映射，不得把 JSON 文本通道加回产品入口。临时调试用 `dump_frame`，不恢复 HTTP JSON。
