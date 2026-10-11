# Design — session 写占用：内核 flock，不要 UUID

## 已定

1. 砍协议 `writer_token` / `X-Writer-Token`（r1793 破坏性改写）。
2. 防双写 = 对该 session 已有文件做 **内核排他锁**（POSIX `flock`；Windows 命名内核对象，无额外文件）。
3. 不新增 `session.lock`：锁 **`manifest.json`**（目录已有、0 额外 inode）。尚未物化的 session 在第一次写出 manifest 时再拿锁（对标 dsh：未物化无足迹）。
4. 「移除锁」= **关掉持有锁的 fd**。不 unlink；flock 跟 inode 走，删文件反而破坏排他（dsh 已写明）。
5. 占用者是 **Host 进程里该 session 的写引擎**，不是 TUI 进程。TUI Ctrl+C 只是一条客户端连接结束。

## 持有与归还

```
materialize 写引擎 ──flock(manifest)──► 持有
        │
        ├─ 仍有连接 / 回合在跑     继续持有
        ├─ 最后一条连接 *干净退出*  立刻 close fd（Ctrl+C Quit、WS Close 帧）
        ├─ 最后一条连接 *异常断开*  linger T 秒，无重连则 close fd
        ├─ linger 期内重连         同进程仍持 fd → 取消 linger，继续写
        ├─ 他进程已持锁            大声失败（会话正被另一进程写入），不抢
        └─ Host 进程死            内核关 fd，后继立即可拿
```

T（linger）可配置，建议键 `session_write_lock_linger`（时长）。`0` = 任何最后断开都立刻还。默认 **60s**（手误 Ctrl+C 再 `--session` 够用；不是抢别人的锁）。

超时 **不是** opencode stale 抢锁：到期只让 **本 Host** 放下自己的 fd。内核 flock 抢不走活着的他进程；活着卡住的写者一直拿到它退出（防撕 JSONL）。

同 Host 双 TUI：Linux flock 按进程，挡不住 → 槽内 `run_inflight` 仍串行。锁管跨进程；忙管同进程。

订阅（mux）是「连接」计数单位。该 session 订阅者从 1→0：干净关闭立刻还；异常断开 linger。纯 HTTP、从未订阅：回合结束后若订阅者仍为 0，走 linger（不是 Quit）。`SwitchSession` 离开旧槽 = 该连接从旧 session 计数去掉，旧槽若因此归零则按干净离开还锁。

`arm_tool_freeze` 不占写锁；他进程持锁时仍须返回冻结快照（原 r1925 动机保留，不再依赖 UUID 竞态）。

## `run()` 收口（同票，否则 Working 仍会挂）

`host.unary("prompt")` 必须看 `RpcResult.ok`。未准入（含他进程持锁）yield Error 并结束 stream。Busy + 空 follow-up 的非 abort Error → Idle。

## 明确不做

- 协议 UUID、last-writer-wins、按 pid 写锁文件再扫僵尸
- 每次 append 才 flock（挡不住双进程交错回合）
- Maka State Root / Epoch
- 为还锁去 unlink `manifest.json`
