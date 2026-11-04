# Design — 确保口上新二进制替换过期 Host

## 目标

本机 ensure 口（回环默认口 / 仅 `--port` 指定口）在 Host 已监听但版本**旧于**当前客户端时，替换旧 Host：SIGTERM 旧 pid → 等退出 → spawn 当前 binary → 等 `/healthz` ready → attach。版本**新于**客户端时 fail-closed，不杀更新进程。非 xylitol 占用不杀、不 spawn。显式非确保 `--attach` / `--no-ensure-host` 仍 fail-closed。

## 现状

- `crates/xylitol-kernel/src/attach.rs:167-196` `classify_running`：版本不等直接返回错误，提示 `kill <pid> && xylitol serve`。
- `attach.rs:534-558` `ensure_host_with`：`inspect_live` Ready 即 `finish_running`；版本不符返回错误，不替换。
- `crates/xylitol-kernel/src/occupancy/subcommand.rs:81-120` `stop_registered_host`：读注册文件 pid → `request_exit`（SIGTERM）→ 等 5s → 删注册文件。
- `crates/xylitol-serve/src/listen.rs:206-223`：SIGTERM 触发优雅 shutdown。
- `server-core`：`/healthz` body 带 pid + version；注册文件含 url / pid / version / token；`serve stop` 只对注册 pid 发 SIGTERM。
- `attach.rs` 已有 `request_exit`（`occupancy::subcommand`）可复用。

## 决策

- 版本比较：用 `semver` 解析 `CARGO_PKG_VERSION` 与 healthz version；任一解析失败 → 不替换，fail-closed 并给可操作错误。
- 旧于客户端 → 替换；新于客户端 → fail-closed；相同 → 直接 attach（不替换）。
- 构建身份：MVP 只比 version；同 version 不同 git 构建无法区分，不替换。可选 build id 后置。
- 替换协议：
  1. 仅 ensure 口；`should_ensure_host` 为真。
  2. `inspect_live` 得 Ready；比较 version。
  3. 旧 → 取替换锁（`~/.xylitol/serve.replace.lock`，flock）→ 重新探活并确认 pid/version 未变 → SIGTERM → 等 pid 退出（5s）→ 清 stale 注册（若仍在）→ spawn 当前 binary → `wait_until_ready`。
  4. 新 → 返回 fail-closed 诊断（升级客户端 / `xylitol serve stop`），不杀。
  5. 非 xylitol（healthz 无 pid/version）→ 沿用 r1787/r3065，不杀不 spawn。
- 锁与竞态：替换锁串行化多客户端；拿不到锁时重探活，若已是新 Ready 直接 attach；spawn `AddrInUse` 沿用现有“忽略并等待 ready”路径，避免双杀/双 spawn。
- 生命周期：替换后的新 Host 走当前 ensure 策略（`serve.autostart_last_client()`），不继承旧 persistent。需要 persistent 的用户显式 `xylitol serve`。
- 注册文件：旧 Host 优雅退出会删注册；若未删且 pid 已死，替换方删除；新 Host 写入新注册。
- SIGTERM 等待 5s；超时返回可操作错误，不 SIGKILL。
- 平台：`request_exit` 非 Unix 返回不支持；替换路径 fail-closed，不假装成功。
- attach 后协议版本不符仍按既有 fatal，不重试风暴。

## 版本比较

```rust
let own = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok();
let peer = semver::Version::parse(&hz.version).ok();
match (own, peer) {
    (Some(own), Some(peer)) if peer < own => Replace,
    (Some(_), Some(_)) => NoReplace, // same or newer
    _ => NoReplace,                  // unparseable -> fail-closed
}
```

- `0.0.0-pre` 是合法 semver；pre-release 排序按 semver。
- 不引入字符串前缀/字典序比较。

## 诊断

- 旧 Host 替换失败：
  - SIGTERM 失败 / pid 不退 / spawn 失败 / ready 超时 → 分类型可操作错误。
  - 不得只提示 `kill <pid> && xylitol serve`。
- 新 Host：`Host runs newer xylitol <v>; upgrade this client or run xylitol serve stop`。
- 非 xylitol：沿用“端口被非 xylitol 占用”诊断。
- 显式 attach / no-ensure：沿用既有 fail-closed 诊断。

## Spec 变更（propose 时落地）

- `cli-entry` r1391 增加场景：
  - 旧 Host 在确保口被替换后 attach。
  - 新 Host fail-closed，不杀。
  - 非 xylitol 占用不杀不 spawn。
  - 显式 `--attach` / `--no-ensure-host` 不替换。
- `user-experience` r3065 增加场景：替换失败 / 客户端过旧诊断分开且可操作。
- `server-core`：沿用 `serve stop` SIGTERM 语义；如新增替换锁，补锁语义条款（或放 `cli-entry`）。

## 测试

- 单测：version compare old/new/same/unparseable。
- `ensure_host_with` wiremock：Ready old version → 替换路径（fake spawn / fake request_exit）；Ready new → fail-closed；非 xylitol → 不杀。
- `request_exit` 测试：dummy pid / 不存在 pid；不误杀当前进程。
- 锁测试：两客户端并发只一个替换；第二个等到新 Ready。
- 手测：旧 binary 起 Host → 新 binary 开 TUI → 自动替换；反向 → fail-closed；非 xylitol 占用 → 诊断；`--attach` → 不替换。
- 门禁：`just fmt`、`just test`、相关 clippy、`just qa`。

## 风险

- 版本字符串 `0.0.0-pre` 与未来 release 比较；semver 解析必须严格。
- 同 version 不同构建不替换；用户仍可能手停。
- 替换会断开仍连在旧 Host 上的客户端；诊断须说明。
- 双客户端竞态 / EADDRINUSE；锁 + 重探活 + 忽略 AddrInUse。
- persistent 生命周期不继承；文档写明。
- 非 Unix 不支持替换；fail-closed。
