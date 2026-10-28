# Tasks

- [ ] 1. **specs**（spec）：`agent-workflow` 改为工具写入并删除 `<plan>` 块条款；`agent-session-store` 记录工具写入同一 CustomEntry；`agent-llm-projection` 记录 POST 不含原始控制块。测：`llman-sdd validate --all --strict`。
- [ ] 2. **工具**（plugin）：新增 `workflow_write` / `workflow_read`；校验 DAG 与 replan；成功 append CustomEntry 并发布 `WorkflowUpdated`；失败不写。测：合法/非法/replan/事件。
- [ ] 3. **中间件**（plugin）：删除 `<plan>` 解析；`after_tool` 采用工具结果或记录错误；`before_model` 注入 `workflow_write` 契约与 `<workflow_error>`。测：采用/错误注入一次。
- [ ] 4. **事件与投影**（sdk/wire/tui）：新增 `WorkflowUpdated` 事件与映射；provider POST 不含原始块。测：事件往返与契约文本。
- [ ] 5. **BDD**（tests）：模型调用 `workflow_write` 后进入步骤指令。测：`cargo test --all-features --test bdd`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
