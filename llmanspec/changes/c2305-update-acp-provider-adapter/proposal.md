---
depends_on:
  - c2301-update-stable-wire-protocol
  - c2302-update-host-multi-session
---

# 产品位：ACP 外部客户端接入（provider 外层适配器）

> **状态：purpose-draft** —— 只钉产品行为与接入定位，**不含技术实现、不列技术选项**。属于后置迭代项：排在 native 协议稳定 + host 多会话落地之后。
> 依据：`c2280/research/acp-interop.md`（一手调研）；`c2301`（native 契约唯一真源、ACP 不入核心闭集）。

## Why

外部 ACP 客户端（Goose / 桌面 / GitHub Copilot 类）以 **ACP（Agent Client Protocol）** 接入 xylitol host 核心，把 host 作为 agent 提供方驱动。调研确认：生态已大（Copilot=provider、Zed=client、Goose=参考实现），且 xylitol host 地基（session registry / Command-Event dispatch / journal / 反向 RPC / WebSocket）已具备，做成"外层适配器"成本可控、并可复用 Rust SDK 1.0 做互操作锚。

## What Changes

产品位：

### 角色与定位

- xylitol host 是 **ACP provider（Agent 侧）**；外部客户端（Goose / 桌面 / Copilot 类）是 **client**。
- 实现为**外层适配器**：把 ACP JSON-RPC method 翻译成 native `Command`，把 native `Event` 译为 ACP `session/update` 推送；**不复制 / 不引入第二套核心语义**——native 契约仍是唯一真源（c2301）。

### 语义覆盖（产品级）

- 一即可用面：会话（new / load / resume / list / close）、消息（`session/prompt` → 流式 `session/update`）、审批（`request_permission` / 问答）、取消（`session/cancel` / `$/cancel_request`）——与 native **近 1:1**。
- 三处必桥（适配层负责，非 ACP 原生）：
  1. `last_seq` 精确续传 / `ResyncRequired` —— ACP v1 断线不重放；v2 才有流续传；
  2. steer / follow-up / 队列 —— ACP v1 无队列原语，v2「beyond the turn」才对得上；
  3. **单写者 + 只读第二 attach** —— ACP 无此概念（允许多 client 观察同一 session），适配层必须自己强制写者锁，并把"只读 attach"表现为可恢复但写被拒。
- 可选面（client 侧能力，不必须）：文件系统、终端、结构化提问，视客户端是否需要再暴露。

### 版本与时序（产品级判据）

- **起步 v1**（稳定）：够基本会话 + 审批；若外部客户端要 xylitol 的 steer / 强续传，再评估 **v2**（Draft，未稳定）或适配层自实现。
- **时序：后置**——排在 native 协议稳定（c2301）+ host 多会话（c2302）落地之后；不并入核心 change。
- **传输**：WebSocket-only 起步（免 HTTP/2 负担；Streamable HTTP 面向 serverless 负载均衡场景，非必选）。

## 非目标（本 change）

- 不改 native 协议闭集、不影响 native TUI 路径（c2301/c2303 语义不变）。
- 不做 ACP client 侧（不消费其它 provider）。
- 不在核心地基（c2301–c2304）完成前开工。

## 开放决策（开工前再定）

- 是否真有外部消费者（谁会连 xylitol host 的 ACP 端口）。
- v1 起步还是直接 v2（承载 steer / 强续传的成本与稳定度取舍）。
- 哪些 client 侧能力面（fs / terminal / elicitation）要暴露。

## 后续（依赖方向，draft）

- 依赖 native wire（c2301）与多会话 host（c2302）；开工前消解上文「开放决策」。
