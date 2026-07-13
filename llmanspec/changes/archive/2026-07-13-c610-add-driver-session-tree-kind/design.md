# Design — c610 SessionTreeKind

## 决策：提前抽象（kind 枚举）

拒绝无修饰 `get_tree` / `navigate_tree` 上抬到 Driver。采用：

```text
session_tree(kind: SessionTreeKind) -> tree
travel_session_tree(kind, entry_id) -> SessionTreeTravel
```

首版仅 `SessionTreeKind::MessageHistory`。其它 variant 可存在于枚举或后续追加；调用未实现 kind → `Err`，禁止 silent fallback。

## 为何不在本 change 做「通用 Tree 注册表」

第二棵树（文件浏览、工具轨迹等）的节点模型与 travel 语义大概率不同。kind 枚举只固定 **Driver 调度入口**；每 kind 的 DTO / travel 字段可在实现分支内演进（`SessionTreeTravel.editor_text` 主要服务 MessageHistory）。

## MessageHistory travel（对齐 pi / demo c600）

| 选中 | leaf | editor_text |
|---|---|---|
| user 消息 | 父 entry（根 user → 约定空叶/根策略） | 该 user 正文 |
| 其它 | 选中 id | `None` |

底层 leaf 写入可继续用 `SessionManager::branch` / `set_leaf`；**不要**把 Driver 方法命名成 `navigate_tree`。

## REST 草图（实现时可微调，须文档化）

- `GET /api/v1/session/{id}/trees/message-history`（或 `?kind=message_history`）
- `POST /api/v1/session/{id}/trees/message-history/travel` body: `{ "entry_id": "..." }` → travel JSON

## 与 infra 旧名

`SessionManager::get_tree` / `navigate_tree` 可保留为实现细节或后续 quick 改名；Driver 对外契约以本 design 为准。
