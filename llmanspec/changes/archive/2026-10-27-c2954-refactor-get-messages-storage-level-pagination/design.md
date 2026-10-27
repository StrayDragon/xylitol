# Design

## 目标

`get_messages` 的分页语义（`c2953` 已定）保持不变，只把「先全量读盘再切片」换成「按需读取窗口内段」。

## 接口

### `XySessionStore`

新增默认方法：

```rust
async fn load_entries_page(
    &self,
    session_id: &str,
    limit: Option<usize>,
    before_entry_id: Option<&str>,
) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
    let entries = self.load_entries(session_id).await?;
    Ok(crate::session::paginate_entries(entries, limit, before_entry_id))
}
```

默认实现保证所有既有 store（测试 mock、debug fixture）不改也能编译；只有持久化后端覆盖为段级跳过。

### `SessionManager`

`read_entries_page` 按 backend 分派：

| backend | 行为 |
|---|---|
| `InMemory` | 内存切片（`paginate_entries`） |
| `Persisted` + manifest | `page_from_segments`（段级跳过） |
| `Persisted` + legacy jsonl | 既有 `legacy_unsupported_error` |
| `Persisted` 无 manifest（pending） | 内存切片 |

### `page_from_segments`

段顺序：`sealed_segments`（旧→新）+ `active_segment`。

```text
1. limit 缺省 → 读全部段后切片（与既有语义一致）
2. before_entry_id 缺省 → 从 active 尾部向前读，直到凑够 limit
3. before_entry_id 给定 → 定位锚点所在段与段内位置
   - 锚点等于某段 first_entry_id → 该段不取，直接从前一段继续
   - 锚点等于某段 last_entry_id → 读该段，取锚点之前
   - 其它 → 有 sidecar 先查 entry_ids（不读段）；命中才读段并定位
   - 找不到 → 空页（与 paginate_entries 一致）
4. 从锚点向前收集，凑够 limit 即停；窗口外的更早段不读
5. has_more = 窗口之外仍有更早条目
```

`limit == 0` 单独处理：返回空页，`has_more` 按锚点全局位置是否 > 0 计算。

## 等价性

`page_from_segments` 的输出必须与 `paginate_entries(load_entries(), limit, before_entry_id)` 逐条一致，包括：

- 顺序（时间正序）；
- `limit` 缺省全量；
- 未知锚点空页；
- `has_more`。

用表驱动单测覆盖多种 `(limit, anchor)` 组合。

## 读取计数

`SessionManager` 在 `#[cfg(test)]` 下持有 `Arc<AtomicUsize>`，`read_segment` 每次自增；测试可重置并断言「最新一页只读活动段」。生产构建不包含该字段。

## 不做什么

- 不改 manifest / segment / sidecar 布局。
- 不做 sidecar 字节偏移表（`seek` 读取）——段级跳过已消掉主要读放大。
- 不改 `get_messages` 的协议形状或分页语义。
- 不改 `load_entries` / `load_leaf_branch` 等既有读取路径。
