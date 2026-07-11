# Tasks — c520-update-xy-event-extensibility

- [ ] 1. Review `XyEvent` / `XyChunk` / `protocol::Event` 变体，确认无厂商专名
- [ ] 2. 在 `src/domain/lifecycle.rs` 顶部加短注释指针到本 change `design.md`
- [ ] 3. 确认 adapter 只产出 `XyChunk`，不直接拼厂商事件进 `XyEvent`
- [ ] 4. （可选 future 记录）Extension 变体列入 `future.md`，本变更不实现
- [ ] 5. `llman sdd validate c520-update-xy-event-extensibility --strict --no-interactive`
- [ ] 6. 若有注释/小改：`just lint`
