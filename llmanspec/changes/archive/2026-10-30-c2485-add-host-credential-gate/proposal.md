---
depends_on: []
title: Host 回环凭据门禁
needs_specs_change: true
branch: sdd/c2485-add-host-credential-gate
base_branch: main
base_sha: 3568f6c249f5727bbb00bebe58a1b2949155d165
---

# Host 回环凭据门禁

## Why

当前监听器（127.0.0.1）无任何鉴权：同机任意进程/用户都能驱动 agent 读盘、执行命令。
writer lease 只闸「并发写身份」，不是访问控制。个人单用户机器上这是真实攻击面
（恶意 npm postinstall、CI 脚本等均可探测固定端口）。

通行做法是对本地回环也上凭据门禁——凭据随机生成、跨重启稳定、经发现契约携带给
合法客户端；WS 等无法带 header 的场景用查询参数兜底；并有一条硬规则：**租约凭据
不得出现在模型执行的工具子进程 env 里**（coding agent 场景下模型会执行任意命令）。

## What Changes

- **产品默认启用**：`xylitol serve` 首次启动在 `~/.xylitol/serve.token`（0600）生成
  随机凭据并跨重启复用；注册文件 `~/.xylitol/serve.json` 增加可选 `token` 字段。
- **门禁范围**：`POST /rpc`、`WS /rpc` 升级与 `GET /healthz` 均要求
  `Authorization: Basic <user>:<token>`；未携带或凭据不符返回 401 +
  `WWW-Authenticate`。WS 升级额外接受 `auth_token` 查询参数作为浏览器兜底。
- **客户端发现**：产品 TUI attach 从与目标 URL 匹配的注册文件读取 token，并在
  healthz 探活与 WS 握手时携带；401 给出可操作诊断（重启 Host / 同账号）。
- **硬规则**：保留凭据环境变量（`XYLITOL_SERVE_TOKEN` / `XYLITOL_AUTH_TOKEN`）
  在 bash、检索/进程工具、交互 bash、外部插件、hooks 与 MCP 子进程 spawn 前剥离。
  凭据只经 0600 文件与注册契约传递，不得写入日志或会话。

## 决策

- **产品 `serve` 默认开启**，不提供「未配置即放行」的产品默认；嵌入式/测试直接使用
  `xylitol_serve` 的 `serve` / `bind_listen` 且不显式给凭据时保持无鉴权（非产品入口）。
- 凭据跨重启稳定，不通过 CLI 旗标或环境变量暴露。
- 不做多用户/远程公网安全模型，不引入账号体系。

## Capabilities

- `server-core`：回环凭据门禁、注册契约携带凭据、401 语义。
- `domain-security`：工具子进程凭据剥离硬规则。

## Impact

- `crates/xylitol-protocol`：Basic auth 编解码与保留环境变量常量。
- `crates/xylitol-serve`：凭据文件、注册文件 token、鉴权 hoop。
- `crates/xylitol-kernel`：产品 `start` 默认生成凭据；attach 探活与 token 读取。
- `crates/xylitol-app-tui`：WS 客户端携带 Basic auth。
- `plugins/xylitol-official-plugin-tools`、`xylitol-kernel`、`hooks`、`mcp`：
  子进程 spawn 前剥离保留凭据环境变量。

## Further Notes

一手对照（外部实现的鉴权中间件、凭据脱敏规则摘录）：
[research/credential-gate-notes.md](./research/credential-gate-notes.md)。
