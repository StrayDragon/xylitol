# rmcp 3.8 prompts/resources/subscriptions 证据

## 结论

- client peer 提供 `listen(SubscriptionFilter)`，返回 `Subscription`；`Subscription::next` 消费 `ResourceUpdatedNotification` / `ResourcesListChangedNotification`。
- `SubscriptionFilter` 支持 `resources_list_changed` 与 `resource_subscriptions: Vec<String>`。
- prompts/resources 的 list/get/read 已由 c3070 接入；本 change 只补 UI、缓存与订阅。

## 证据

- `rmcp-3.8.0/src/service/client.rs:1284`：`pub async fn listen(&self, notifications: SubscriptionFilter)`。
- `rmcp-3.8.0/src/service/client.rs:419`：`Subscription` 句柄与 `next()`。
- `rmcp-3.8.0/src/model.rs:2242`：`SubscriptionFilter` 字段。
- `rmcp-3.8.0/src/handler/client.rs:57`：`ResourceUpdatedNotification` 分发。
