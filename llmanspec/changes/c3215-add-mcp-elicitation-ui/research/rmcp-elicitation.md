# rmcp 3.8 elicitation 证据

## 结论

- `ClientHandler::create_elicitation` 接收 `ElicitRequestParams`，返回 `ElicitResult`。
- `ElicitResult` 含 `action`（accept / decline / cancel）与可选 `content: Value`（须符合 schema）。
- `ElicitationSchema` 的 primitive 子集为 String / Number / Integer / Boolean / Enum；string 支持 email / uri / date / date-time。

## 证据

- `rmcp-3.8.0/src/handler/client.rs`：`create_elicitation` 回调。
- `rmcp-3.8.0/src/model.rs:4073`：`ElicitResult` 字段。
- `rmcp-3.8.0/src/model/elicitation_schema.rs:54`：`PrimitiveSchemaDefinition` 变体。
- `rmcp-3.8.0/src/model/elicitation_schema.rs:96`：`StringSchema` 字段与 format。
