---
depends_on: []
needs_specs_change: true
branch: sdd/c3035-add-ensure-host
base_branch: main
base_sha: ad5e3f9dbf771933dbb11afd9022c71d7131f378
---

# 开箱确保本机 Host（探活即连，否则拉起，last-client 销毁）

## Why

产品 TUI 默认 attach 本机 Host，未在听就失败并要求用户先跑 `xylitol serve`。这和「开箱即用」冲突：一次 `xylitol` 应该能进 TUI。

现行硬闸是 `cli-entry` `tui-attach-fail-closed`（r1391）与 `app-tui-preview` designing 同等 fail-closed（r2906）。TUI **仍必须**经信封 attach，不得改走同进程直握（r1182 保持）。缺的是「没有 Host 就确保一个」这条开箱路径，以及自动拉起实例的生命周期。

## 决策（探索已拍板）

- **默认策略 B：确保存在 + last-client。** 探活到健康本机 Host 就连；没有则拉起独立 Host 进程再 attach。自动拉起的实例在最后一个客户端断开并经过 idle grace 后自行退出。
- **显式 `xylitol serve` 仍是常驻（persistent）。** Ctrl-C / `serve stop` 才停，不被 last-client 杀掉。这是今天的语义，也是切到策略 C 的逃生口。
- **策略 C 是同一套 Host 上的策略翻转，不是第二套架构。** C = 自动拉起也用 persistent（用户级 daemon）。本 change 把生命周期做成 Host 侧策略枚举；C 只需改自动拉起的缺省策略（配置或后续 change），不得再抽一套进程模型。
- **不做 GitHub FR。** 仓库主人变更；roadmap #28 不强制挂条。

## What Changes

- 产品 TUI（TTY 默认进入、`xylitol tui`、debug `tui designing`）在 attach 目标是本机回环默认口（或仅 `--port` 的回环口）且 Host 未在听时，**必须确保**一个 Host 再 attach，不得再要求用户先手起 `xylitol serve`。
- 确保顺序：探活 → 健康 xylitol 则连；僵死注册则清理后拉起；端口被非 xylitol 占用或版本不符则仍 fail-closed（保持 r1787 / r3001 的分级诊断）。确保口上新二进制替换旧 Host 不在本 change，见搁置草案 `c3040-add-stale-host-replace`。
- 拉起方式：以当前可执行文件 spawn 独立 `serve` 进程（脱离 TTY / SIGHUP），仍走既有 Kernel::boot + 监听 + `serve.json`。TUI **不得**在本进程直握操作器，也不得把 HTTP 监听嵌进 TUI PID（否则 C 无法零成本切）。
- 双开竞态：两个客户端同时确保时，输家遇 EADDRINUSE 必须改等就绪再 attach，不得把「地址占用」当成开箱失败。
- 自动拉起的 Host 默认生命周期为 last-client：已认证 WS `/rpc` **从 1 降到 0** 后启动 idle grace，到期自行优雅退出并删除注册文件；**从未有过连接不得因 idle 退出**。存活判据是 WS 连接本身，不靠应用层请求或轮询心跳。缺省 grace 约 30 秒，可配置。
- 显式 `xylitol serve` 默认 persistent。`xylitol serve stop` 必须按注册文件 pid 发送 SIGTERM（取代现在的「请自己 kill」提示）。
- 显式 `--attach <url>`（非「解析出来的本机默认口」）保持 fail-closed，不自动拉起——用户指向了特定 Host。
- Print / 库嵌入仍同进程，不在本 change。
- `server` feature 关闭时保持今天的 fail-closed（无法在本二进制内拉起 Host）。

## Capabilities

- `cli-entry`：改写 r1391（fail-closed → 本机默认口确保 Host）；r1388 表面旗标如需 `--no-ensure-host` 等再扩。
- `server-core`：注册文件可携带生命周期策略；last-client 自毁；`serve stop` 真停；双开确保重试。
- `app-tui-preview`：designing 与产品 TUI 同一套确保语义（改 r2906）。
- `user-experience`：确保失败时的可操作诊断（拉起失败 / 就绪超时），承接 r1787 分级，不再只说「请先 serve」。
- `app-tui-bridge` r1182 **不改**：默认仍是信封 attach，同进程路径只给 print / 嵌入。
- `runtime-config`：可选新增 `serve` 配置块（lifecycle / idle_timeout），作为 B→C 的产品开关。

## Impact

- `xylitol-kernel` `attach`：preflight 之上加 ensure（探活 / spawn / 等 ready / EADDRINUSE 重试）。
- `xylitol-serve`：连接计数 + last-client idle 自毁；Registration 可选字段；`serve stop`。
- 根包 CLI：TUI 入口从「失败即退」改为「确保再 attach」；`serve` 子命令接受生命周期旗标。
- `xylitol-official-plugin-config` / schema / 示例配置：`serve` 块。
- 根 `AGENTS.md`、`src/AGENTS.md`、`README`：开箱路径从「先 serve 再 tui」改为「`xylitol` 即可」；显式 serve 仍是常驻。
- BDD：`cli-entry` / `server-core` / `app-tui-preview` 场景与步骤。

## 非目标

- 不把产品 TUI 改回同进程直握。
- 不取消「禁止 port+1」（r1797 / r1801）。多 TTY 共享同一回环口。
- 不实现用户级 daemon 安装（systemd --user / launchd）。C 只是 persistent 策略，不是服务管理器。
- 不给每个 TUI 私有端口（`--standalone` 换口）。单口共享是 B 的前提。
- 不改 Print 去 attach 共享 Host（OpenCode `run --attach` 是后续，可在 C 时做）。
- 不改会话写占用 / linger / journal 语义；last-client 数的是 **WS 连接**，不是 session 槽。
- 不在本 change 把默认策略切到 C。
- 不在本 change 做确保口「新二进制替换旧 Host」（搁置 `c3040-add-stale-host-replace`）。

## 不承诺

- idle grace 的精确缺省秒数可在实现时按手感微调（提案口径「约 30 秒」）；产品必须可配置，且 `false` 表示自动拉起也 persistent（即本机切到 C）。
- Windows 脱离控制台的具体 API（`DETACHED_PROCESS` 等）由实现选择，行为是「TUI 退出后 Host 仍能活到 last-client」。

## Open Questions

已解决：

- 默认生命周期 = B（ensure + last-client）。显式 serve = persistent。C = 同一进程模型上把自动拉起改为 persistent。
- 内建 ≠ 同进程直握；内建 = TUI 确保独立 Host 进程。

推迟到 C / 后续：

- 是否提供 `xylitol service start|stop|status` 别名（OpenCode 风格）。本 change 只把 `serve stop` 做真。
- Print 是否可选 attach 已有 Host。
- idle 是否要避开进行中回合（v1 只看连接数，到期走既有优雅停机）。

## Further Notes

OpenCode 对照摘录见 `research/opencode-host-lifecycle.md`。关键差：OpenCode 默认已是 C（共享 daemon 不随最后客户端退出）；xylitol 本 change 默认 B，架构按能切 C 来铺。
