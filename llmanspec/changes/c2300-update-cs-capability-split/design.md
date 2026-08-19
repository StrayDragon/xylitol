# Design：client / host 角色与后续票缝

本票只落地**产品角色**合约。监听器、载体、闭集仍按波次实现。

## 1. 三个词，不要混用

| 词 | 含义 | 谁落地 |
|---|---|---|
| **client** | 面本地：键、画、TTY、编辑器、剪贴板 | 本票合约；TUI 现状已基本如此 |
| **host（操作器角色）** | 模型 / 会话 / MCP / 工具 / 工作区 / trust | 本票合约；默认仍在同一进程里 |
| **listener（`serve` 监听器）** | 占用某个绑定的 HTTP 服务器 | **不是** host 的定义。c2302 组合根 + c2303 CLI |

embed：client + host 同进程，**无** listener。
attach：client 连已有 listener 后面的同一个 host 角色。
禁止再说「host = 占了 18790 的那个进程」——那是 listener。

## 2. embed 接到契约的产品缝（后续票必须遵守）

```text
client  --Command/Event-->  host dispatcher
              ^                     |
              |                     +-- 工作区 / 模型 / MCP / session
              |
     carrier 可变：同进程 channel 或 HTTP 升级 WebSocket
     语义不可变：禁止进程内走 Driver 旁路、远程走另一套 REST 产品语义
```

- **本票钉死**：只有一条产品语义（Command / Event → 同一 dispatcher）。
- **本票不钉**：channel crate、是否 loopback WS、类型名。那是 c2301（闭集）+ c2302（传输面）+ c2303（进线）。
- c2302 **禁止**做成「只有套接字才进 dispatcher」，否则 c2303 会再开进程内旁路。

## 3. 现行 live spec 冲突表（本票不改右列）

| 现行 MUST | 目标 | 改它的票 |
|---|---|---|
| `layer-architecture` la6：server 单实例锁；REST+WS 暴露契约 | listener 用绑定占用（EADDRINUSE），不整机锁；REST 不承载产品语义 | c2302（锁/绑定）+ c2301（REST 退出产品语义） |
| `server-core` sr3/sr7/sr8：锁文件 + port+1 | 占用即失败；`--port 0` 由 OS 分配 | c2302 / c2303 |
| `protocol-app` ip3/ip9：REST 信封；Approve 留 WS 层 | 一切产品语义都是 Command；dispatch 可执行 | c2301 |
| `cli-entry` ce6/ce8：InProcess/Remote 双 Driver；`server run/install/stop` | 薄客户端 + `serve` / `--attach` | c2303 |
| `app-tui` tui2/tui3：默认 InProcessDriver；三面 REST+WS 共存 | 默认仍是同进程 embed（可继续叫 InProcess）；REST 产品面后撤 | tui2 本票保留；tui3 随 c2301/c2302 |

本票只 **新增** 角色合约，让默认单进程路径仍然为真，因此本票可独立校验、独立归档。新条款已写明：`la-cs3` **不**废止 `la6` 的锁/REST；导出/导入/reload 与一写者在未提供 attach 时由同进程路径满足。后续票改 `la6` / `server-core` 时再收旧 MUST，避免读者把角色合约读成「现在就必须拆进程或拆掉 REST」。

## 4. 绑定与 Docker（用户已裁）

- 默认听 `127.0.0.1`。用户可显式 `--host 0.0.0.0`（自己负责暴露面）。**不**学 DSH 拒绝 `0.0.0.0`。
- **本波不做** Docker 粗沙盒（容器内听、发布端口、镜像文档）。需要时另票。
- 落地在 c2303 的 `serve --host/--port`；本票只把决策锁进交接，避免 c2303 再写成「拒绝 0.0.0.0」。

## 5. 测试边界（seam）

复用已有 harness，不发明脱离 `.feature` 的新边界：

| 测什么 | 怎么测 | 不测什么 |
|---|---|---|
| 新合约可解析、不与旧 MUST 对打 | `llman sdd validate c2300-update-cs-capability-split --strict` | 新 CLI 子进程 |
| 默认 TUI 仍是同进程 embed | 现有 `app-tui.feature` `@req:tui2`（InProcessDriver） | 多窗写者、`--attach` |
| 运行时无回归 | 现有 `just qa` / `cargo test --test bdd` | 本票不新增可执行 `.feature`（多客户端行为尚未存在） |

新 requirement 的例子放在 `spec.toon` 的 `feature: false` 审查行（与现行 `layer-architecture` 同型）。一写者 / 导出路径等在未提供 attach 前由「单进程默认路径自然满足」成立，不把未实现的 HTTP host 写成可执行 GWT。
