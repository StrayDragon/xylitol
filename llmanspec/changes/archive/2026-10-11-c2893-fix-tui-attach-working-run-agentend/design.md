# Design — session 写占用：内核锁，不要 UUID

## 已定

1. 砍协议 `writer_token` / `X-Writer-Token`（r1793 破坏性改写）。
2. 防双写对齐 deepseek-harness：已物化 session 目录里的 **`session.lock`** 是锁的**稳定名字**（空文件、从不改写、从不 unlink）。**不**锁 `manifest.json`（那份文件会 tmp+rename，换 inode，Windows 还会和打开句柄打架）。
3. 尚未物化的 session 无足迹；第一次写出 manifest 后再取锁。
4. OS 分叉（同一套占用语义，不同内核对象）——dsh 已如此：
   - **POSIX**（Linux / macOS）：对 `session.lock` `flock(LOCK_EX|LOCK_NB)`。锁跟 inode 走；取锁后核对 fd 的 ino/dev 仍是路径上那份，否则重开（防 unlink+重建）。还锁 = close fd。
   - **Windows**：不 `LockFileEx` 任何会话文件。用规范化锁路径（大小写折叠）的 SHA-256 派生命名信号量 `Local\xylitol-session-lock-{hex}`（`CreateSemaphoreW` count=1，零超时 `WaitForSingleObject`）。还锁 = ReleaseSemaphore + CloseHandle。最后 handle 关时对象销毁，后继可重建。
5. 占用者是 **Host 进程里该 session 的写引擎**，不是 TUI 进程。TUI Ctrl+C 只是一条客户端连接结束。
6. 同进程第二写者（含同 Host 双 TUI）内核锁挡不住——Linux flock / Win 信号量都按进程。同进程靠槽内 `run_inflight`（dsh 的 `claimWrite` Map）。锁只管跨进程。

## 持有与归还

```
materialize 写引擎 ──POSIX flock(session.lock) / Win 命名信号量──► 持有
        │
        ├─ 仍有连接 / 回合在跑     继续持有
        ├─ 最后一条连接 *干净退出*  立刻还（Ctrl+C Quit、WS Close 帧）
        ├─ 最后一条连接 *异常断开*  linger T 秒，无重连则还
        ├─ linger 期内重连         同进程仍持内核对象 → 取消 linger，继续写
        ├─ 他进程已持锁            大声失败（会话正被另一进程写入），不抢
        └─ Host 进程死            内核关 fd / 销毁未引用信号量，后继立即可拿
```

T（linger）可配置，建议键 `session_write_lock_linger`（时长）。`0` = 任何最后断开都立刻还。默认 **60s**（手误 Ctrl+C 再 `--session` 够用；不是抢别人的锁）。

超时 **不是** opencode stale 抢锁：到期只让 **本 Host** 放下自己的内核对象。活着卡住的写者一直拿到它退出（防撕 JSONL）。

订阅（mux）是「连接」计数单位。该 session 订阅者从 1→0：干净关闭立刻还；异常断开 linger。纯 HTTP、从未订阅：回合结束后若订阅者仍为 0，走 linger（不是 Quit）。`SwitchSession` 离开旧槽 = 该连接从旧 session 计数去掉，旧槽若因此归零则按干净离开还锁。

`arm_tool_freeze` 不占写锁；他进程持锁时仍须返回冻结快照（原 r1925 动机保留，不再依赖 UUID 竞态）。

## `run()` 收口（同票，否则 Working 仍会挂）

`host.unary("prompt")` 必须看 `RpcResult.ok`。未准入（含他进程持锁）yield Error 并结束 stream。Busy + 空 follow-up 的非 abort Error → Idle。

## 明确不做

- 协议 UUID、last-writer-wins、按 pid 写锁文件再扫僵尸
- 每次 append 才取锁（挡不住双进程交错回合）
- Maka State Root / Epoch
- 锁 `manifest.json`；为还锁去 unlink `session.lock`
