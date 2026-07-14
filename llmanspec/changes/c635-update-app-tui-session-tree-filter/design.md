# Design — c635-update-app-tui-session-tree-filter

## Locked（相对 pi / demo）

| 主题 | 决议 |
|---|---|
| 五种模式名 | 对齐 pi：`default` / `no-tools` / `user-only` / `labeled-only` / `all` |
| Ctrl+D | 直接 → `default` |
| Ctrl+T/U/L/A | **toggle** 该模式 ↔ `default`（对齐 pi；**不**跟 demo 的纯 set） |
| Ctrl+O | cycle forward：default → no-tools → user-only → labeled-only → all → … |
| Shift+Ctrl+O | **本 change 不做**（MAY 后续） |
| 状态行后缀 | `default` **无**后缀；其它才 `[no-tools]` / `[user]` / `[labeled]` / `[all]`（对齐 pi；demo 的 `[default]` 不抄） |
| 包边界 | 产品谓词经 `include_node` + `status_suffix`；包不硬编码 FilterMode |

## default vs all（对齐 pi）

| 模式 | 可见 |
|---|---|
| **default** | 排除 bookkeeping：`kind == "meta"`（见下） |
| **no-tools** | default 且 `kind != "tool"` |
| **user-only** | `kind == "user"` |
| **labeled-only** | `annotation.is_some()` |
| **all** | 全部节点 |

**bookkeeping → `kind=meta`**（mapper）：`ModelChange` / `ThinkingLevelChange` / `Label` / `SessionInfo` / `Custom` / `CustomMessage` / `Header`。
`Message` / `BashExecution` / `Compaction` / `BranchSummary` 等保持既有 kind 或无 meta。

## Mapper

- 既有：`id` / 正文 `label` / role→`kind`
- **新增**：`SessionTreeNode.label` → `TreeNode.annotation`（域已 resolve；产品此前丢掉）
- meta 类 entry：`kind = "meta"`（主题可不画前缀）

## 键抢占

树开（`EditorSlot::Tree`）时 Ctrl+T / Ctrl+O 走 filter；关树后仍为 thinking / 工具视口（与 pi/demo 一致）。

## Non-goals

- fold / fork / annotation 编辑
- Settings `treeFilterMode` 持久化
