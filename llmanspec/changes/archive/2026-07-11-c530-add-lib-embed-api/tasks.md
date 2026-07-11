# Tasks — c530-add-lib-embed-api

- [x] 1. 选定导出载体（`embed` mod vs feature）并写入 design 定稿一句
- [x] 2. 将 `app::core` 中嵌入所需符号对库用户可见；`lib.rs` 文档化清单
- [x] 3. 更新 `docs/architecture/library-and-clients.md`「嵌入方应依赖」与现状表
- [x] 4. （可选）最小 doc-test / 示例：bootstrap → run 一轮 mock
- [x] 5. `llman sdd validate c530-add-lib-embed-api --strict --no-interactive`
- [x] 6. `just lint` + 相关测试
