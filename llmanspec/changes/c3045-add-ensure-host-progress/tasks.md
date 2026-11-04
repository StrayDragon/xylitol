# Tasks

## 1. BootProgress 模块

- [ ] 1.1 新增 `crates/xylitol-app-tui/src/boot_progress.rs`：`BootProgress` / `with_boot_progress`；TTY 门禁、150ms 延迟、80ms 帧、清行。
- [ ] 1.2 在 `crates/xylitol-app-tui/src/lib.rs` 导出；不引入新依赖（std + tokio）。
- [ ] 1.3 单测：快完成不输出、慢完成输出、finish 清行、非 TTY 不输出、writer 失败静默降级。

## 2. CLI 接入

- [ ] 2.1 `src/cli.rs` `should_ensure_host` 分支包 `BootProgress`；`attach_preflight` 分支不包。
- [ ] 2.2 确保 `progress.finish()` 在 trust gate 之前；失败先清行再 `eprintln!` 既有诊断。
- [ ] 2.3 覆盖 `--no-ensure-host` / 显式非确保 `--attach` / 非 TTY / `server` feature 关闭：不启动进度。
- [ ] 2.4 确认 `TerminalGuard::enter` 仍在 ensure + trust gate 之后；不提前 raw / alt-screen。

## 3. 测试与规格

- [ ] 3.1 CLI 预检单测：Ready / 超时 / spawn 失败；断言进度清行、诊断仍可操作。
- [ ] 3.2 `cli-entry` r1391 增加三个场景：可观察、Ready 不闪、失败清行。
- [ ] 3.3 `user-experience` r3065 增加失败清行场景。
- [ ] 3.4 手测：无 Host / 已有 Host / kill Host / 端口占用 / `--no-ensure-host`。
- [ ] 3.5 `just fmt`、`just test`、相关 clippy、`just qa`。

## 4. 收口

- [ ] 4.1 确认无新增产品画布 / 假 Busy / 假 Reloading。
- [ ] 4.2 确认 print / 嵌入 / 非 TTY 行为不变。
- [ ] 4.3 用户确认后再开 PR；确认前不 push。
