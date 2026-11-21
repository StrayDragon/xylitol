# 调研底稿：xylitol 三轴基线判定与证据

三路仓库探索（crates / plugins+根包 / specs+工具链）与针对性实证（2026-11-20，工作树快照）。

## 一、体量与分层

- crates/ 9 crate：456 文件约 16.3 万行；plugins/ 17 crate：180 文件约 4.8 万行；根包 src 12 文件 2.6 千行；tests/ 151 文件 5.4 万行。
- 五层：契约叶（sdk 75 文件 14.1k、protocol 10 文件 2.3k、tui 引擎 61 文件 28k）→ 官方插件（最大 provider-openai 7.3k、mcp 6.3k、tools 5.4k）→ 执行权威（runtime 79 文件 24.4k，run_react_loop 于 agent/runtime/react/mod.rs:896；kernel 43 文件 12.2k，boot.rs:57）→ 载体与端（serve 7.4k、wire 655 行、app-tui 59.5k、app-print 537 行）→ 根包（cli.rs 995 行）。
- specs：59 capability 目录 + 1 扁平 feature；BDD 82 个 feature、1,204 处场景绑定；归档 change 130 个目录（另有 152 个旧式 yaml 快照）。

## 二、三轴支撑判定

| 轴 | 判定 | 地基 | 缺口 |
|---|---|---|---|
| actor 化 | 支持 | 会话槽单写者（r1514）、Command 词表、XyEvent、rutis 事件面 | r2914 / r2921 合约、occupancy 手写状态机 |
| rutis 插件化 | 地基支持 | 生命周期 + 19 端口 + 注册表（工具 / 中间件 / XyTurnInjector loop_middleware.rs:201 / 服务 / ReloadHub） | 贡献面无合约化清单；外进程桥只认工具 |
| 数据结构 | 大半支持 | append-only（session log append/commit_compaction/create_with_fork）；注入机制最成熟（context-inject 五维注册表 registry.rs:15-40） | 版本等值硬门（parse.rs:42-49）；未知行 compaction 不保（待验证）；不可变性无合约条目 |

## 三、关键实证

- 会话解析：行级 unknown 容忍（skip + 告警，parse.rs:17-24），enum 无 Unknown 变体（未知行被丢弃不保留）；header 版本等值硬校验（parse.rs:42-49）；LEGACY_SESSION_VERSION 迁移路径现存（parse.rs:53-67，属待删历史债务）。
- 无 rewind 面（全仓 grep 零命中）；label / edge / checkpoint 均追加式。
- 方法表 50 条 unary，MethodEntry 含 exec: Exclusive|Inline|Queued 元数据（protocol registry.rs:5-39）——三通道语义的现成落点。
- 协议下行通知 8 种；XyEvent 与 wire 逐变体显式映射（wire event.rs 652 行）。

## 四、已定决策（2026-11-20 与维护者确认）

1. 一步到位、不兼容、无双路径；旧会话数据不迁移（等值门拒载即文档化）。
2. busy 拒绝改信箱排队；新增逐项撤销（对话通道）；bang 保持 Exclusive 拒绝；slash 控制保持 Inline。
3. 插件单一脊柱：rutis 进程内插件 + MCP 统一外部工具；发现与委派保留、清单增 mcp 声明；私有工具桥退役。
4. 双库 lockstep：thoughtdag 直接适配，只有一种新模式，不设兼容层。
5. 全组 change 在独立 worktree（xylitol.actor-rewrite-refactor）进行，避开主线 apply。
