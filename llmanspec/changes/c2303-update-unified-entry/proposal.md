---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2302-update-host-multi-session
---

# 产品位：统一启动与连接（embed 默认 + 显式 attach）

> **状态：purpose-draft** —— 只钉产品行为，**不含技术实现、不列技术选项**；实现细节（socket 发现、握手、锁落地）列「开放决策」。
> 依据：`c2300`（启动与连接方式、host 唯一、多窗写入规则）、`c2301`（契约传输面）、`c2302`（host 多会话组合根 + 契约传输面）。

## Why

c2300 定义了统一 CS 的形态（默认 A、host 每机器/容器唯一、attach 显式、多窗写入规则）。本 change 把它落成两条**用户实际可用的启动与连接路径**：默认单命令（体感与今天一致）与显式 `--attach`（连已运行 host）；并保证**面本地始终在 TUI 侧**，不因 embed / attach 而分叉。

## What Changes

### embed 默认（体感不变）

- 默认单命令 = 与今天等价的体验：同一进程内嵌 host，客户端经契约自连；用户**无需先启动 server**。
- 进程内的 embed host 仍是「每机器唯一 host」的形态（c2300/c2302）：同一机器上的其它 TUI 可以 attach 到它（它照常暴露契约传输面）。
- 第一次运行的启动体验应与今天无感知差异（不因架构变化增加冷启动负担、不要求用户先起 serve）。

### attach 显式

- `--attach` 显式连接一个已运行的 host（同机走本地连接；远程 / 容器走网络，见 c2302 传输面）。
- serve 未运行 → attach 失败并给出明确提示（告知如何启动 host），**禁止**静默回退到某个"第三模式"。
- 连接即订阅（c2301 journal/续传语义）：断线重连按契约恢复。

### 面本地在 TUI 侧（两种进线下都成立）

- 剪贴板（含 OSC 52）、TTY、`$EDITOR`、键位、绘制均在 TUI 进程执行（c2300 归属）；不因 embed / attach 改变归属。
- 非面本地能力一律经契约到 host（c2301）。

### 多窗与读写（c2300 规则落地）

- 对**已被其它 client 写入**的 session attach → **只读静态恢复**视图（非 tmux 共屏写）。
- 对只读 session 发起写（对话） → host 拒绝并给出「有其他 TUI 已连接，当前仅只读」（c2301 契约语义）。
- host 可表达每 session 的连接状况（谁在写、谁只读）。

### 生命周期

- 关一扇 TUI 不终止 host；host 停止则所有连接的 TUI 断开（c2300）。
- 停 host = 优雅停机（在途收摊 + attach 全断，c2302）。

## 开放决策（设计阶段再填）

- embed 的「最后一扇窗关闭」后 embed host 的去留（随 TUI 退出启动它的宿主，还是保持供别机 attach）。
- attach 发现机制（本地 socket 路径 / 端口 / 握手与版本协商）。
- 锁语义落地（P3：一台 host 一把锁；第二 serve 拒绝），与 c2302 transport 分工。
- 首启自动 attach 还是纯显式（c2300 已定显式；这里细化命令行形态）。

## 非目标（本 change）

- 不做协议闭集本身（c2301）、不做 host 多会话组合根（c2302）、不做符合性闸（c2304）、不做 ACP（c2305）。
- 不做面本地的协议化（c2300/c2301 已定不进协议）。

## 后续（依赖方向，draft）

- 承载于 c2302 host 组合根；c2304 用本 change 的两条进线做双实现同跑验证。
