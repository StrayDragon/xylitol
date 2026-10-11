# Tasks

测试缝：Host unary + mux 订阅计数（既有 `tests/bdd/steps_server.rs` / `bindings_server.rs` `writer-lease`）；`SessionManager` 持久化打开/追加（`agent-session-store` 既有 create-load）；`XyRemoteDriver::run` 与 `apply_xy_event`（`remote.rs` / `src/app/tui/bridge`）。不新开 harness。不测真 TTY。跨进程冲突用本机第二 Host 或 flock 争用夹具，不连真网关。

## T1 — 持久化内核写占用

- [ ] 已物化会话：对 manifest 取内核排他锁（POSIX flock / Windows 等价内核对象）。不新建锁文件，不 unlink。
- [ ] 同进程可续持；他进程争用失败。进程退出后后继可取。
- [ ] 校验：`agent-session-store` 单测或 BDD——两打开写冲突；持有方 drop 后第二打开成功。

## T2 — [blocked-by: T1] Host 砍令牌 + 连接生命周期

- [ ] 去掉 `take_writer_lease` / `X-Writer-Token` / 应答 `writer_token`。writer 方法不再要令牌。
- [ ] 物化写引擎时取 T1 锁。该 session mux 订阅 1→0：干净关闭立刻还；异常断开 linger（默认 60s，`0` 立即）。linger 内同进程再订阅续占。
- [ ] 纯 HTTP 无订阅：回合结束且订阅者 0 则 linger。`SwitchSession` 离开旧槽按干净离开计。
- [ ] `arm_tool_freeze` 不占写锁，他进程持锁仍返回冻结快照。
- [ ] 校验：改写既有 `writer-lease` / `writer-token-via-ws-unary`；新场景覆盖干净释放与 linger 续占。

## T3 — [blocked-by: T2] prompt 未准入结束 run + Idle

- [ ] `XyRemoteDriver::run`：prompt unary 非成功 MUST yield Error 并结束 stream。
- [ ] bridge：Busy 且 follow-up 空的非 abort Error → Idle，Error 行保留。
- [ ] 校验：remote 单测「业务错误后 stream 结束」；`config_error_is_sticky_error_row` 扩 Idle；`busy-lifecycle-until-agent-end-headless` 仍绿。

## T4 — [blocked-by: T2, T3] BDD

- [ ] `server-core` / `agent-session-store` / `app-tui-bridge` / `protocol-app` 本票场景绑到既有 steps。
- [ ] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（仅本票相关场景）。

不在本任务列 finalize。
