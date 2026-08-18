---
depends_on: []
branch: sdd/c2300-update-cs-capability-split
base_sha: 7acd591a41afffa741a7a1ea199ec53701e3aa6b
checkpointed: true
checkpoint_sha: 7acd591a41afffa741a7a1ea199ec53701e3aa6b
---

# 统一 client / host 能力归属

TUI 恒为薄客户端。host 是操作器角色，不是「必须先占住一个端口」。默认 embed = 同进程、同一契约自连；显式 `serve` 才占用绑定。证据：`c2280` `02-split`、`research/overhead-eval.md`。

## Why

逻辑全在终端进程，且进程内 / 远程两套语义。先钉**谁做什么**，再拆协议、监听器与进线。本票只交归属合约；不换 HTTP 栈、不改线协议闭集、不接线 `--attach`。

## What Changes

### 双角色（本票 SSOT）

- **client**：键盘、绘制、TTY、`$EDITOR`、剪贴板（含 OSC 52）；发 Command、画 Event。
- **host（操作器角色）**：模型、会话生命周期、资源装配（配置 / MCP / 工具 / 技能 / prompt）、工作区执行（含 `!` / `!!`）、`/trust`。
- 一条 `/reload`：client 热加载键位与主题；同时向 host 发 reload，重装 MCP / prompt / 技能。
- 导出：client 发命令 → host 序列化 → 经契约回传内容 → client 写本机路径。导入对称。

判据：依赖工作区 / 仓库 / 模型 / MCP → host；依赖本机终端 / 硬件 → client。

### 词汇（本票钉死，实现可后置）

- 最小单元是 **session**。`cwd` 是执行面，不是唯一分类夹。视图可用 **tag** 等分面；分面索引与 transcript 分离。tag catalog 另票。
- 一 session 一写者。尚未提供多客户端 attach 时，单进程默认路径自然满足。

### 与后续票的交接（本票不实现）

- **监听器**（占用 `{addr,port}` 的 HTTP `serve`）：c2302 / c2303。默认 `127.0.0.1:18790`；用户可显式 `--host 0.0.0.0`。本波不做 Docker 粗沙盒镜像/发布约定。
- **embed 接到契约的载体**（同进程 channel vs 套接字）：产品缝见 `design.md`（同一 dispatcher，禁止第二套语义）；实现归 c2301 起。
- REST 停产品语义、整机锁改为绑定占用、协议闭集、符合性闸、ACP：c2301–c2305。

## 非目标

协议闭集、多会话组合根、进线 CLI、符合性闸、ACP、换编码、Web 开闸、自动 attach、后台拉起、引用计数、`serve --stop`、Docker 粗沙盒、tag catalog。不改 `server-core` 现行 REST / 锁文件 / port+1（留给落地监听器的票）。

## Capabilities

- `layer-architecture`：client / host 角色、embed 不要求监听器、session 原子与一写者。
- `app-tui`：产品 TUI 只承担面本地。

## Impact

- 现行 `la6`（server 单实例锁）、`server-core` REST 信封、`cli-entry` Driver 双实现、`app-tui` tui2/tui3 **本票不改**，避免合约超前于代码。后续票按 `design.md` 冲突表改。
- 根 `AGENTS.md` / `src/AGENTS.md` / `docs/architecture/库与多客户端.md` 写入角色心智（理想 vs 现状，不把 attach 写成已交付）。
