# 设计：确保本机 Host

## 分层

| 职责 | 落点 | 理由 |
|---|---|---|
| 探活 / 分级诊断 | kernel `attach`（已有 preflight） | 现行 r1787 入口 |
| spawn + 等 ready + EADDRINUSE 重试 | kernel `attach` / 根包 CLI | TUI 入口在根包，Host 装配在 kernel |
| 监听、WS 计数、idle 自毁 | `xylitol-serve` | 连接真源在监听器；TUI 不得 SIGKILL 自己拉起的 pid |
| 生命周期策略 | Host 进程旗标 + 注册文件字段 | B/C 是同一进程的策略，不是两种拓扑 |
| 配置 `serve.*` | config 插件 | 产品开关；自动拉起读取，显式 serve 缺省忽略 |

TUI 进程只做：ensure → attach → 断开。Host 自己决定何时退出。这是 B→C 的硬缝：C 把「零连接 + grace」改成「永不因零连接退出」，TUI 一行都不用改。

## 策略枚举

```text
persistent   = 显式 `xylitol serve` 缺省；零连接不退出
last-client  = 自动拉起缺省；已认证 WS /rpc 连接数为 0
               且 idle grace 到期 → 优雅退出并删 serve.json
```

旧注册文件无 `lifecycle` 字段 → 视为 `persistent`（兼容今天已在跑的 serve）。

## 开箱流程

```text
解析 attach URL
    │
    ├─ 显式 --attach 且不是「本机默认口 / 仅 --port 的回环口」
    │     → 旧 preflight，失败即退，不 spawn
    │
    └─ 本机默认确保口
          probe + healthz
            ├─ 200 且版本匹配 → attach
            ├─ 端口被非 xylitol 占用 / 版本不符 / 401 无凭据
            │     → 既有分级诊断，不 spawn
            │       （新二进制替换旧 Host：搁置 c3040）
            ├─ 未在听（含僵死注册）
            │     → 清僵死文件（既有 classify_dead）
            │     → spawn current_exe() serve --lifecycle last-client …
            │     → 等 /healthz 200（starting 的 503 要等，不当失败）
            │     → attach
            └─ spawn 侧 EADDRINUSE
                  → 视为他方刚起来，转入等 ready 再 attach
```

`server` feature 关闭：走旧 fail-closed。

## 为何必须是独立进程

Last-client 要跨 TUI 寿命：TUI 1 拉起、TUI 2 连上、TUI 1 退出，Host 必须还在。监听若嵌在 TUI 1 的 PID 里，TUI 1 退出即连坐 TUI 2，且无法切 C。

Spawn 约束：

- 可执行文件 = 当前 `current_exe()`，避免 PATH 上另一份 xylitol 版本漂移（preflight 已拒版本不符）。
- 脱离控制台：POSIX `setsid` / 忽略 SIGHUP；Windows 用脱离控制台的进程创建旗标。stdin 置空。
- 子进程写同一套 `serve.json` / `serve.token`（既有 0600 + 回环 Basic）。
- 就绪窗口遵守 r1786：绑定后 starting、装配完 ready；客户端等 200，有界超时。

## 谁算「一个客户端」

只计已通过凭据门禁的 **WS `/rpc` 连接**（产品订阅通道）。短 unary `POST /rpc` 不计——否则一次 `host.describe` 会重置 idle，也会在无 TUI 时把 last-client 钉死。

计数点复用既有 `register_unbound_mux` / `prune_mux`。从 1→0 启动 idle 计时；0→1 取消计时。**启动时零连接不启动 idle**（避免 TUI 尚未连上或单客户端长期不发 unary 时被误杀）。`persistent` 不启动计时。

存活判据是已认证 WS 本身：产品 TUI 在会话期间持有 `/rpc` mux。协议层 ping/pong 只探测死连接，不是 occupancy 心跳，也不是应用请求轮询。客户端开着但不发 prompt 时 Host 必须仍在。

到期：既有优雅停机（停监听、删注册、30s graceful）。不另做「等回合结束」；进行中回合走停机取消。v1 不把 in-flight 当 keepalive（推迟项）。

## 注册文件

在既有 `{url,pid,version,token}` 上增加可选：

```text
lifecycle: "last-client" | "persistent"
```

未知字段旧客户端忽略。自检（字段不全等则旧进程退出）必须把新字段算进相等性，避免 last-client 实例被一份无该字段的旧文件误杀——写注册的是自己，相等性用完整结构。

## 配置（B→C 开关）

```yaml
serve:
  lifecycle: last-client   # 或 persistent
  idle_timeout: 30s        # 或 false：自动拉起也 persistent（本机切 C）
```

缺省（不写该块）= last-client + 约 30s。仅自动拉起读取。显式 `xylitol serve` 默认 persistent，可用 `--lifecycle last-client` 覆盖。

C 的落地可以是：把缺省改成 `persistent`，或用户写 `serve.idle_timeout: false`。本 change 只把开关铺好，不改缺省到 C。

## `serve stop`

读 `serve.json` pid，存在则 SIGTERM，等退出、确认注册文件消失。pid 不在 / 不是 xylitol → 可操作错误，不乱杀。这是 B 清 grace 残留和 C 停 daemon 的同一入口。

## 与现行不变量的关系

| 不变量 | 本 change |
|---|---|
| r1182 信封 attach | 保持。ensure 之后仍 RemoteDriver |
| r1797 禁止 port+1 | 保持。多 TTY 共享 18790 |
| r1787 注册发现 | 保持并扩展可选字段 |
| r3001 回环凭据 | 保持。spawn 的 serve 写同一 token |
| r1786 starting 503 | 保持。ensure 等待 ready |
| r1391 fail-closed | **改写**为本机默认口 ensure；显式 --attach 仍 fail-closed |
| 「Host ≠ HTTP 监听器」 | 保持。spawn 的是 Host 操作器进程，监听只是载体 |

## 测试缝

复用既有 CLI 子进程 / occupancy 真进程 harness，不另发明协议。

1. **ensure 公共函数**（kernel attach）：无监听 → spawn 后 healthz 200；已在听 → 不二次 spawn；EADDRINUSE → 重试 attach；非 xylitol 占用 → 错误且不 spawn。
2. **CLI 产品入口**：无 Host 时 `xylitol tui` 不再以「请先 serve」非零退出（可用 `--list-models` 或预检到 ready 为止的窄缝，不必完整 TTY 主环）。
3. **Host last-client**：真进程 serve `--lifecycle last-client`，一条 WS 断开后 grace 内仍 healthz 200，到期后进程退出且注册文件消失；grace 内重连则继续活。
4. **Host persistent**：显式 serve，断光 WS 仍活。
5. **`serve stop`**：停掉注册 pid。
6. **显式 `--attach` 非默认口**：不 spawn。
7. **designing**：与产品 TUI 同一 ensure（debug 构建）。

完整 TUI PTY 冒烟保持 `#[ignore]` / `just test-tui-e2e-pty`，本 change 不强制把它拉进 `just qa`。
