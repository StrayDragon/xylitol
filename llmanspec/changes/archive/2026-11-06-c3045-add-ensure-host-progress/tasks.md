# Tasks

## 1. BootProgress 模块

- [x] 1.1 新增 `crates/xylitol-app-tui/src/boot_progress.rs`：`BootProgress` / `with_boot_progress`；TTY 门禁、150ms 延迟、80ms 帧、清行。
- [x] 1.2 在 `crates/xylitol-app-tui/src/lib.rs` 导出；不引入新依赖（std + tokio）。
- [x] 1.3 单测：快完成不输出、慢完成输出、finish 清行、非 TTY 不输出、writer 失败静默降级。

## 2. CLI 接入

- [x] 2.1 `src/cli.rs` `should_ensure_host` 分支包 `BootProgress`；`attach_preflight` 分支不包。
- [x] 2.2 确保 `progress.finish()` 在 trust gate 之前；失败先清行再 `eprintln!` 既有诊断。
- [x] 2.3 覆盖 `--no-ensure-host` / 显式非确保 `--attach` / 非 TTY / `server` feature 关闭：不启动进度。
- [x] 2.4 确认 `TerminalGuard::enter` 仍在 ensure + trust gate 之后；不提前 raw / alt-screen。

## 3. 测试与规格

- [x] 3.1 `BootProgress` 单测覆盖快完成 / 慢等待 / 清行 / writer 失败 / 非 TTY；BDD 覆盖 Ready、超时失败与旧版替换等待。
- [x] 3.2 `cli-entry` 新增 `ensure-host-progress` 需求块（新 `@req`）与三个场景：可观察、Ready 不闪、失败清行；`tests/bdd` 补绑定。
- [x] 3.3 脚本化回归：boot_progress 单测 + BDD 三场景；真 TTY 手测（无 Host / 已有 Host / kill Host / 端口占用 / `--no-ensure-host`）由人执行（未验证）。
- [x] 3.4 `just fmt`、`just test`、相关 clippy、`just qa`。
