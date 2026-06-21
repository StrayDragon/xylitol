# Tasks

- [x] 1. 新增 `Transport` 枚举（auto / sse / direct）（defer）
- [x] 2. 新增 `SteeringMode` 枚举（all / one-at-a-time）（defer）
- [x] 3. 新增 `DefaultProjectTrust` 枚举（ask / always / never）（defer）
- [x] 4. 新增 `MarkdownSettings` 结构体（defer）
- [x] 5. 新增 `WarningSettings` 结构体（defer）
- [x] 6. 在 `Settings` 中新增 16 个顶层 Option 字段（defer）
- [x] 7. 更新 `deep_merge()` 处理新字段（defer）
- [x] 8. 在 `SettingsManager` 新增 accessor 方法：`get_transport()`、`get_steering_mode()` 等（defer）
- [x] 9. 确保 `#[serde(rename_all = "camelCase")]` 正确应用于新类型（defer）
- [x] 10. 编写序列化/反序列化单元测试（defer）
- [x] 11. `cargo test --lib` 全绿（414 passed）（defer）
- [x] 12. `cargo clippy` 无新增警告（defer）
