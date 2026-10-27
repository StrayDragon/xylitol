---
depends_on:
  - c2953-refactor-journal-delta-tape-scope-and-persisted-message-recovery
title: get_messages 分页下沉到存储层，避免整会话读入内存
---

# get_messages 分页下沉到存储层，避免整会话读入内存

## Why

`c2953` 给 `get_messages` 加了 `limit` / `before_entry_id`，但**分页只在内存里切片**：

```rust
// xylitol-kernel/src/driver/command_exec.rs
let all = self.get_messages().await?;                 // ← 全量读盘
let (entries, has_more) = paginate_entries(all, limit, before_entry_id.as_deref());
```

`get_messages` → `XySessionStore::load_entries` → `SessionManager::load_entries`，后者**无条件读取全部 sealed segments + active segment**：

```rust
for segment in &manifest.sealed_segments {
    entries.extend(self.read_segment(session_id, segment).await?);   // 逐段全读
}
entries.extend(self.read_segment(session_id, &manifest.active_segment).await?);
```

后果：

- **首屏 200 条**仍然要把整个会话（可能几十 MB、上千段）读进内存再丢弃；
- 长会话打开变慢、峰值内存与**会话总长度**成正比，而不是与窗口大小成正比；
- `c2953` 的目标（内存可控）只兑现了「TUI 侧窗口 800」，**没有兑现「读取侧只读所需」**。

而所需元数据**已经存在**：

| 字段 | 位置 | 作用 |
|---|---|---|
| `sealed_segments[].first_entry_id` / `last_entry_id` | `SessionManifest` | 段的首尾条目 id |
| `sealed_segments[].generation` | `SessionManifest` | 段序号 |
| `sealed_segments[].index_path` | `SessionManifest` | 段旁 sidecar 索引 |
| `SealedIndex.entry_ids` / `done_bash_ids` | sidecar JSON | 段内条目 id 全表 |

也就是说：**存储格式不需要改**，只是读取路径没用这些信息。

## What Changes（候选项，promote 时逐条重估）

### 1. 段级跳过

`load_entries` 增加分页变体（或让 `XySessionStore` 新增 `load_entries_page`）：

```text
目标：从最新一条向前取 limit 条
1. 从 active_segment 尾部开始读
2. 不够 limit → 向前遍历 sealed_segments（逆序）
3. 若该段 last_entry_id 已满足窗口起点 → 读该段后切片即停
4. 若整段在窗口之外 → 用 first/last_entry_id 直接跳过，不 read_segment
```

判据：`before_entry_id` 落在哪个段。段描述符已有 `first_entry_id` / `last_entry_id`，可**不读文件**就判断「该段是否整体在窗口之后 / 之前 / 跨越窗口」。

### 2. 段内精确定位（可选，第二阶段）

若某段跨越窗口边界，当前实现仍需整段读入再切片。进一步可用 `index_path` sidecar 的 `entry_ids` 做二分，或给 sidecar 增加**字节偏移表**（`entry_id → byte offset`）以支持 `seek` 读取。

**这是可选优化**：段级跳过已能消掉绝大部分读放大（长会话通常几十段，只有 1 段跨越边界）。

### 3. 契约面

- `XySessionStore` 的 `load_entries` **保持不变**（既有调用方语义不变）。
- 新增带分页的读取入口；`get_messages` 分页走它。
- `has_more` 由存储层给出（比内存切片更准：内存切片只能知道「本页之外还有没有」，存储层知道**盘上**还有没有）。

### 4. 回归保障

`c2953` 已有的分页语义**必须逐条保持**：

| 语义 | 来源 |
|---|---|
| `limit` 缺省 → 全量 | `r1699` |
| 从最新一条向前取 | `r1699` |
| `before_entry_id` 取更早条目，页间不重叠 | `r1699` |
| 未知 `before_entry_id` → 空页（不猜窗口） | `paginate_entries` 单测 |

须有等价性测试：**分页读取结果 == 全量读取后切片结果**（对同一会话、同一参数）。

## Out of Scope

- 改会话存储格式（manifest / segment / sidecar 布局）。
- 改 `c2953` 已定的分页参数（200 / 800）。
- `get_messages` 之外的其他读取路径（`load_leaf_branch`、`session_tree`、`export`）。
- sidecar 字节偏移表（列为 §2 可选优化，独立评估）。

## Capabilities

- `agent-session-store`：存储层分页读取、段级跳过、`has_more` 语义。
- `protocol-app`：`get_messages` 分页参数语义**不变**（仅底层实现变化，无契约改动）。
- `server-core`：`r1810` 分页条款不变，可能补「读取不得整会话载入」的性能约束。

## Impact

- crate：`xylitol-plugin-sdk`（`XySessionStore` 新入口）、`xylitol-official-plugin-session`（段级跳过实现）、`xylitol-kernel`（`command_exec` 改走存储分页）、`xylitol-runtime`（in-process 驱动同步）。
- 测试：分页等价性（存储分页 == 全量切片）；段边界跨越；整段跳过不读文件（可用读取计数断言）；既有 `c2953` 分页 BDD 回归。
- 文档：`agent-session-store` spec 补分页读取约束。

## Further Notes

来源：`c2953` apply 时明确记录的留待项（见该 change `proposal.md` Out of Scope 与本轮报告）。

**可复现的现状证据**：

```text
plugins/xylitol-official-plugin-session/src/manager/segments.rs:190-205
  load_entries(&self, session_id, manifest) -> Vec<SessionEntry>
  遍历 manifest.sealed_segments 全部 read_segment，再读 active_segment

crates/xylitol-kernel/src/driver/command_exec.rs:103-120
  let all = self.get_messages().await?;
  let (entries, has_more) = paginate_entries(all, limit, before_entry_id.as_deref());
```

**为什么现在不做**：`c2953` 已把用户可见语义（分页、窗口、提示行）落地且门禁全绿；存储层下推是**纯性能优化**，不改变任何外部行为，适合独立 change 单独评估与验证（尤其需要读取计数类断言，避免「以为跳过了其实没跳过」）。

**验证手法建议**（promote 时定）：给 `SessionManager` 注入读取计数器（仅测试用），断言「取最新 200 条时只读 active segment（+ 至多 1 个 sealed segment）」，而不是读全部段。
