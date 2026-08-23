---
depends_on: []
---

# Serve 启动就绪窗口：三态健康语义

## Why

`xylitol serve` 绑定端口到装配完成（trust / MCP / provider 就绪）之间存在启动窗口；
产品 TUI 默认 attach 固定端口，窗口期内的请求只能得到笼统失败，客户端无法区分
「还在启动，稍等重试」与「启动失败，别等了」。业界成熟服务在同窗口提供
starting / ready / stopping / failed 分态应答 + `retry-after`，由服务端主导退避节奏，
客户端体验明显更好（外部对照见 research 笔记）。

## What Changes

- 监听器引入就绪状态机：starting → ready；stopping / failed 为终态分支。
- 窗口内 `GET /healthz` 单独应答：503 + 语义化状态码 + `retry-after`；
  failed 给不可重试语义。
- 未 ready 时其余 unary / WS 升级请求统一 503（带同语义码），不半执行。

## 非目标

- 不改变「显式 serve 占绑定、TUI 未在听即失败」的产品模型。
- 不做健康检查鉴权（属 c2485 凭据门禁范围）。

## Impact

- `src/app/server/runtime.rs` / `http.rs`：绑定后先挂最小应答器，装配完成后切换路由。
- `/openapi.json` 文档随动；BDD 补「启动窗口内请求得到 503 + retry-after」场景。

## Further Notes

- 外部实现对照笔记：[research/readiness-window-notes.md](./research/readiness-window-notes.md)
