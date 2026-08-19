---
depends_on: []
---

# 产品位：统一 CS 能力归属

TUI 恒为薄客户端。host 是占用某个绑定（默认 loopback HTTP）的服务器，不是整机互斥锁。默认 embed = 同进程、同一契约自连；`--attach` 连已在听的 host。证据：`c2280` `02-split`、`research/overhead-eval.md`。

## Why

逻辑全在终端进程，且进程内 / 远程两套语义。收敛为一条契约、一套归属。

## What Changes

### 双端

- client：键盘、绘制、TTY、`$EDITOR`、剪贴板（含 OSC 52）；发 Command、画 Event。
- host：模型、会话生命周期、资源装配（配置 / MCP / 工具 / 技能 / prompt）、工作区执行（含 `!` / `!!`）、`/trust`。
- 一条 `/reload`：TUI 热加载键位与主题；同时向 host 发 reload，重装 MCP / prompt / 技能。
- 导出：客户端发命令 → host 序列化 → 经协议传回内容 → 客户端写本机路径。导入对称：客户端读本机文件，把内容发给 host。

依赖工作区 / 仓库 / 模型 / MCP → host；依赖本机终端 / 硬件 → client。

### 启动与连接

- host = HTTP 服务器（WebSocket 升级承载契约）。默认 `127.0.0.1:18790`；Docker 发布该端口。`--port` 覆盖。
- 同一绑定已被占用 → 新 `serve` 失败（EADDRINUSE）。换端口 / 换地址可再起另一个 host。
- 默认 embed：同进程自连，契约与 attach 相同。
- `--attach` 连已在听的 host。
- 关掉一个客户端不停止 HTTP host；host 进程停则连在它上面的客户端断开。

### 会话原子与分面

- 最小单元是 **session**：一次对话的全部 turn 记在这一条里。
- `cwd` 是执行面（工具 / bash / trust / MCP 池），不是唯一分类夹。
- 视图用 **tag** 等高频分面组织（画像式读写）。分面索引与 transcript 分离，不靠重写 JSONL 正文。
- 一个 HTTP host 可持有跨 cwd 的多条 session。

### 多窗写入

- 一 session 一写者；host 追踪每 session 连接数。
- 对已被写入的 session attach → 只读静态恢复。
- 只读上写 → 「有其他 TUI 已连接，当前仅只读」。

## 非目标

协议闭集、多会话、进线、符合性闸（c2301–c2304）。Web 不开闸。自动 attach、后台拉起、引用计数、`serve --stop` 不进本 change。tag catalog（id → tags/cwd/title/mtime，与 JSONL 分离）另票，不挡本波。
