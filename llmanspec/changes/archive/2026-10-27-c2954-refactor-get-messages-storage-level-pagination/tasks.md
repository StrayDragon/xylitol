# Tasks

- [x] 1. **spec 要求**（spec）：在 `agent-session-store.feature` 新增 `r2938`「会话条目分页读取」：分页入口、与全量切片等价、段级跳过、`has_more` 语义；注明段级跳过由存储层单测（读取段计数）覆盖。测：`llman sdd validate c2954-refactor-get-messages-storage-level-pagination --strict`。
- [x] 2. **store 端口**（sdk）：`XySessionStore` 新增 `load_entries_page` 默认实现（回退到 `load_entries` + `paginate_entries`）。测：`cargo test -p xylitol-plugin-sdk`。
- [x] 3. **持久化分页**（session）：`read_entries_page` 按 backend 分派；`page_from_segments` 实现段级跳过与锚点定位；`#[cfg(test)]` 读取段计数。测：新增 `manager/tests/pagination.rs` 的等价性与读取计数单测。
- [x] 4. **驱动入口**（runtime）：`XyInProcessDriver::get_messages_page(limit, before_entry_id)` 走 `load_entries_page`。
- [x] 5. **kernel 接线**（kernel）：`Command::GetMessages` 改走 `get_messages_page`，不再先全量 `get_messages` 再切片。测：`cargo test -p xylitol-kernel --lib dispatch`。
- [x] 6. **回归与门禁**（repo）：既有 `get_messages` 分页 BDD 保持绿；`just qa` 全绿。测：`cargo test --all-features --test bdd test_sr_get_messages_paging` + `just qa`。
