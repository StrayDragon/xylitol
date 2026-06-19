# Tasks

- [x] 1. 新增 CLI 子命令 `resources`（clap subcommand：`list` / `info <name>` / `doctor`）
- [x] 2. `list`：复用 `DefaultResourceLoader`，输出 skills/prompts/themes（含 scope + path + frontmatter 状态）
- [x] 3. `info <name>`：查找并输出单个资源详情；未找到返回非零退出码
- [x] 4. `doctor`：输出 `ResourceDiagnostic` 列表；有问题返回非零退出码
- [x] 5. 审计命令路径确保全部只读（不创建/修改/删除任何文件）
- [x] 6. 单元测试：list 格式 / info 命中与未命中 / doctor 干净与有问题退出码
- [x] 7. 集成测试：放置资源文件 → `list` 发现 → `doctor` 报告
- [x] 8. Run `cargo test --lib`
- [x] 9. Run `cargo fmt` 与 `cargo clippy`
- [x] 10. Run `llman sdd validate c120-add-resource-diagnostics --strict --no-interactive`
- [x] 11. Run `just qa`
