# rmcp 3.8 capability probe 证据

## 结论

- client peer 提供 `discover(meta)`，返回 `DiscoverResult`。
- `DiscoverResult` 含 supported versions / capabilities / instructions / ttl_ms / cache_scope。
- 高层 client 只在 initialize 后暴露 discover；pre-initialize 探测未支持。

## 证据

- `rmcp-3.8.0/src/service/client.rs:1380`：`pub async fn discover(&self, meta)`。
- `rmcp-3.8.0/src/model.rs:1474`：`DiscoverResult` 字段。
- `rmcp-3.8.0/src/service/client.rs:1376`：注释说明 pre-initialization probing 为后续工作。
