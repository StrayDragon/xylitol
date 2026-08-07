# src/app/ 应用面

入口索引。分层 / seam / 应用面状态：**只**在 `src/AGENTS.md`。全局与 AGENTS 写法：根 `AGENTS.md`。

| 任务 | 去哪 |
|---|---|
| 新增或改造应用面 | `write-surface` |
| 死代码分诊 | `audit-dead-code` |
| TUI 产品面 | `tui/AGENTS.md` + `write-tui`；刻意差异 → `tui/PI_DELTAS.md` |
| TUI 包内验证 | `test-tui-harness`；包 `AGENTS.md` |
| steer / follow-up | `XyDriver` 队列；产品语义见 `docs/architecture/插话续跑与中止.md` |
| 产品 slash 目录 | `app::product_commands`（SSOT）+ `XyDriver::get_commands` |
| bang / session export | API 在 `XyDriver`；实现迁往 `app/core`（勿再扩 `agent::capabilities`） |
