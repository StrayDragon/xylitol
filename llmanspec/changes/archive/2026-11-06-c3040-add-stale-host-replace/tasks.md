# Tasks

> 阶段目标：planned（proposal + design + tasks）。apply 前须 `change start` 并落地 specs。
> Seam：`xylitol_kernel::attach::ensure_host_with`（注入 spawn / request_exit / registration_path / ready_timeout）；BDD 用 `tests/bdd/steps_cli_surface.rs` 的 `AttachBdd` 纯决策 harness。

## 1. 版本判定

- [x] 1.1 根 `Cargo.toml` 增 `semver` workspace 依赖（lock 已有 1.0.28）；`crates/xylitol-kernel/Cargo.toml` 引用。
- [x] 1.2 `attach.rs` 增 `VersionVerdict` 与 `version_verdict(own, peer)`；单测覆盖旧 / 新 / 相同 / 非法串 / 预发布排序。
- [x] 1.3 `classify_running` 改用判定：旧版提示将被替换，新版 fail-closed，非法版本提示不可比较。

## 2. 替换路径

- [x] 2.1 `ensure_host_with` Ready 分支按判定分流：相同 → attach；新于或不可比较 → fail-closed；旧于 → 替换。
- [x] 2.2 替换锁：`<registration_path>.replace.lock`，Unix `flock` 独占；拿锁后重探活，pid 或版本变化则重判。
- [x] 2.3 SIGTERM 旧 pid（复用 `occupancy::subcommand::request_exit`）；50ms 轮询 `pid_alive` 至多 5s；超时不 SIGKILL，返回含 pid 的可操作诊断。
- [x] 2.4 旧 pid 退出后清理仍指向旧 pid 的注册文件；spawn 当前 binary，`AddrInUse` 走 detached 并等 ready。
- [x] 2.5 非 Unix 不替换，返回平台不支持的可操作诊断（仅编译期分支，未在非 Unix 实机验证）。

## 3. 测试

- [x] 3.1 `ensure_host_with` + wiremock + 注入 spawn / request_exit：旧版替换、新版不杀、非 xylitol 不杀不拉起、非法版本不替换。
- [x] 3.2 锁：并发两次调用只执行一次替换，另一个等新 ready。
- [x] 3.3 `request_exit` 与 pid 等待：不存在 pid、已退出、超时；不误杀当前进程。
- [x] 3.4 替换后注册文件：旧注册被清理，新注册指向新 pid。

## 4. 规格与 BDD

- [x] 4.1 `cli-entry` r1391：规则句补版本分流；新增「旧版被替换」「新版不被杀」「非 xylitol 不杀不拉起」「显式 attach 不替换」四个场景。
- [x] 4.2 `tests/bdd/steps_cli_surface.rs` 补对应步骤，`bindings_cli_entry.rs` 绑定。
- [x] 4.3 `user-experience` r3065：规则句补替换失败与客户端过旧；新增两个诊断场景与绑定。

## 5. 手测与门禁

- [x] 5.1 脚本化回归：`cargo nextest run -p xylitol-kernel --all-features attach` 与 `cargo nextest run --all-features --test bdd` 覆盖替换 / fail-closed / 非 xylitol / 显式 attach；旧 binary → 新 binary 的 TUI 手测由人执行（未验证）。
- [x] 5.2 `just fmt`、`just lint`、kernel 与 BDD nextest、`just qa`。
