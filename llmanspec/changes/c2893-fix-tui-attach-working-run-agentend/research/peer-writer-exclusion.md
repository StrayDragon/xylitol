# 同级 harness 怎么做「防双写」

日期：2026-10-11。一手来源：本地 `../deepseek-harness`、`../maka`，以及同级 `crush` / `opencode` / `pi` / `crystalith` / `kimi-code`。对照 xylitol `WriterLease`（内存 UUID，无释放）。不是 live spec。

## 结论

别人要防的是 **撕日志 / 两个 Host 写同一份状态**，锁绑在 **内核 fd（进程死了锁没了）**。xylitol 防的是 **RPC 信封上谁可以调 writer 方法**，锁绑在 **Host 进程里一张不会过期的 UUID**。同名业务，落点反了：我们把「连接身份」做成了「所有权」，又忘了还牌。

个人 coding harness **需要**「同一 session 同时只有一个 ReAct 写者」。**不需要** sticky `writer_token`。deepseek-harness 的 flock、Maka 的 State Root OS lock，都比 UUID 更接近根因。

## 对照

| | 锁什么 | 仲裁者 | 持有期 | 进程/连接死了 | 冲突时 |
|---|---|---|---|---|---|
| **xylitol** | session 槽上 `writer_token` 字符串 | Host 内存 Mutex | mint 后直到 Host 重启 | **不释放** | `writer_conflict`；RemoteDriver 还把它当 prompt 成功 |
| **deepseek-harness** | 该 session 目录 `session.lock` | 内核 `flock` / Win 信号量 | 写句柄一生（agent-loop create/resume→dispose close） | fd 关 → 锁没；卡死进程故意不抢（防撕日志） | `SessionAlreadyOwnedError`，读仍可开 |
| **Maka** | 整个 State Root | 内核 `tryLock`（`root-authority.ts`） | Host 进程写权威 | close：拒新操作 → 等在途 → 放 OS handle | `Runtime Host root is already owned` |
| **crush** | spawn 单实例文件 | `flock` | 拉起过程 | close 解锁 | 阻塞等到拿到 |
| **opencode** | 通用 `locks/` 文件 | 文件锁 + **staleMs=60s** 抢锁 | 按 key | 过期可抢（dsh 明确拒绝这种抢法） | 等/超时 |
| **crystalith / kimi-code / pi** | 未见 session 写者 UUID 租约 | — | — | — | — |

### deepseek-harness（`packages/session/session-persistence-jsonl/src/lease.ts`）

- 文件头写死：仲裁者是内核；崩溃持有者永不挡后继；**没有 expiry**（活着但卡住的持有者一直拿到进程退出，因为剥夺会撕 log）。
- `open(id,'write')` 原子声明；`read` 从不占有。进程内还有 `claimWrite` Map，跨进程靠 flock。
- agent-loop 是生产环境唯一取写句柄的点；teardown `close` 才释放（`packages/core/agent-loop/README.zh.md` §写句柄）。
- 浏览器 stub flock=成功：单进程，进程内 claim 已互斥。

### Maka（`docs/architecture/runtime-host-architecture.zh-CN.md` §3、§5、§10）

- **一个 State Root 同时最多一个 writer Host**。Desktop/TUI/CLI 是入口，不本地再 compose Runtime。
- 写入权威来自稳定文件上的 OS lock，不是 PID/socket/healthz（删发现缓存不能变出第二个 writer）。
- **两层**：`SessionAdmissionGate` = 每 session **短临界区**，执行离开 admission 后不占用；`RootTurnCoordinator` = 每 session 同时一个 root execution。不是「连接拿着牌直到下次登录」。
- Client 断线释放 connection-scoped subscription / controller lease，**不**自动取消已准入工作。

### 同级其它

- crush：`acquireSpawnLock` 排他 flock，防双开启动，不是 session JSONL。
- opencode：`packages/util/src/flock.ts` 带 stale 抢锁——和 dsh「禁止 expiry 剥夺」相反。
- xylitol JSONL：`src/infra/session/mod.rs` 写了「Atomic appends with file locking」，实现里只有内存 `RwLock`，**没有 flock**。协议租约也护不住两个 Host 进程写同一 `~/.xylitol/sessions/`。

## 对 xylitol 的含义

真不变量（`src/AGENTS.md` 单写者）：绑定 session 后至多一个 ReAct worker 写它。Host 槽已有 `driver` + `run_inflight`。

假不变量：信封 UUID 证明「这个 TCP/WS 客户端是写者」。HTTP 每次新连接（r1793 自己说连接不是身份），WS 却把 token 抄进连接本地又抄进槽——槽在连接死后仍占着。这不是防双写，是防「死人复活」。

TUI attach 本机 serve 的真实风险：

1. 同一 Host、两个客户端 prompt 同一 session → 槽内 `run_inflight` / 队列就能串行，不必 UUID。
2. 两个 Host 进程写同一 JSONL → 只有 flock（dsh）或 Root lock（Maka）能防；UUID 看都看不见。
3. TUI 退出再 attach → 今天被 UUID 误伤；内核锁下这是合法后继。

## 已定（2026-10-11）

砍协议 UUID；防双写用系统锁。内存锁治标不治本。回写 `proposal.md` Open Questions。

## 可选方向（未拍板）

| | 做什么 | 不做什么 |
|---|---|---|
| **删协议租约** | writer 方法不再要 token；同槽第二 prompt 看 `run_inflight`（忙则拒或入队）；连接死了下一客户端就是写者 | 不搬 Maka 全套 State Root / Epoch |
| **dsh 式 flock** | 写开 JSONL 时 `flock` session 目录；Host 进程死自动还 | 不引入 stale 抢锁（opencode）；不抢活着的卡住写者 |
| **只修释放** | 保留 UUID，WS close 清牌 | 仍是协议层补丁；两个 Host 仍能撕日志 |

个人开箱、单 serve：优先 **删 UUID + 槽内互斥**；若 Print/第二 serve 能碰同一 JSONL，再加 flock（与协议正交）。`run()` 必须看 `RpcResult.ok` 是另一层：没开跑就不得等 AgentEnd。

## 来源

- `../deepseek-harness/packages/session/session-persistence-jsonl/src/lease.ts`
- `../deepseek-harness/packages/session/session-persistence/src/index.ts`（open write 合约）
- `../deepseek-harness/packages/core/agent-loop/README.zh.md`
- `../maka/docs/architecture/runtime-host-architecture.zh-CN.md`
- `../maka/packages/storage/src/root-authority.ts`（`tryLock`）
- `../crush/internal/cmd/spawnlock_other.go`
- `../opencode/packages/util/src/flock.ts`
- xylitol `src/app/server/host.rs` `take_writer_lease`；`src/infra/session/mod.rs`
