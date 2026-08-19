---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
---

# 产品位：host 多会话组合根 + 契约传输面

> **状态：purpose-draft** —— 只钉产品行为，**不含技术实现、不列技术选项**；框架选型 / listener 拓扑 / MCP 池 key 等实现细节列「开放决策」，由设计阶段填充。
> 依据：`c2300`（host 每机器/每系统运行时唯一、多窗写入规则、A 默认）、`c2301`（稳定线协议闭集）、`c2280`（现状：`Mutex<driver>` 单实例单会话 + REST 不承载产品语义）。

## Why

现状 host 是"一把锁 + 一个合成会话"，无法支撑产品要的 **N TUI ↔ 1 host、一个 host 可承载多会话**（c2300）。本 change 把 host 升级为**多会话组合根**，并承接 c2301 的契约传输面（废弃现有 REST 作为产品语义的临时设计、重建传输），作为统一进线（c2303）与符合性闸（c2304）的宿主地基。

## What Changes

### host 形态（产品级）

- 一条运行中的 host = **一台机器唯一**（或每个系统运行时唯一：Docker 一容器一 host）；启动即宿主，停止则所有连接断开（c2300）。
- host 同时承载**多个 session**；每条 session 遵循 c2300 多窗写入规则（一写者；附加 client 只读静态恢复；host 追踪每 session 连接数）。
- host 侧拥有会话数据与信任：会话归属 host，`/trust` 写入 host 侧项目信任文件（c2300）。

### 多会话语义（产品级）

- 会话生命周期（新建 / 切换 / 命名 / 删除 / 导出 / 导入）按 c2301 语义词面在 host 执行。
- 每条会话的写者唯一：对已被其它 client 写入的 session 发起写 → host 拒绝并给出「有其他 TUI 已连接，当前仅只读」。
- host 决定每次写入的归属与接受与否（单飞语义：同时只有一个写者驱动一次回合）。

### 资源装配与共享语境（产品级）

- 配置 / 模型注册表 / MCP / 工具集按 **host** 装配（不是按窗口）。
- MCP 池发生在 host 进程，key 意向 `(mcp_name, canonical cwd)`（c2280/c2300），多窗共享、避免重复启动税。
- 资源重载（reload）作用于 host 装配语境，所有连接共享。

### 契约传输面（产品级）

- host 提供 **契约传输面**，供多个 TUI（含默认 embed 形态的客户端）连接：客户端发 Command、收 Event，经 c2301 契约。
- 现有 REST **不作为产品语义承载**（c2280/c2301）；传输面按新契约重建，承载产品路径。
- 框架 / listener / 编码等实现细节列「开放决策」。

## 开放决策（设计阶段填充，不在本 draft 钉死）

- **HTTP/server 框架（已选）**：**salvo** 为默认——需求覆盖矩阵见 `research/framework-pick.md`（WS 一等、广播背压文档化、UDS+TCP 双听、sock 权限、优雅停机带 force 超时、auth/origin、静态/CORS、ACP 单端点与 HTTP/2 可后加）；**axum 为保守回退**（首次实现遇文档与代码不符再评估）；按用户要求不做可行性验证，仅文档对比。
- 新传输面的 listener 拓扑与连通细节（本地 UDS、远程 TCP、容器 published port）。
- MCP 池 key 的落地阈值与共享语义。
- 与 embed 默认客户端（c2303）的同进程连接形态。

## 非目标（本 change）

- 不实现 TUI 启动与连接进线（c2303）、不符合性闸（c2304）。
- 不做 ACP 外部接入（c2305 外层适配器）。
- 不做编码二进制化（c2300 已判 tagged JSON 足够）。

## 后续（依赖方向，draft）

- c2303 统一进线（embed + attach）、c2304 符合性闸、c2305 ACP provider 适配器，均承载于本 change 的多会话 host。
