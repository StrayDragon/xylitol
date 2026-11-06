# Design — MCP pending cue 收口

## 目标

- `mcp pending (see /mcp)` 只在 MCP 准备确实未完成时显示；freeze / arm / timeout / reload / resume 后及时收起。
- header、status cue、`/mcp` 面板读取同一份状态，不出现“面板 connected+armed、header connecting”的分裂。
- 挂死 stdio server 在 `connect_timeout_ms` 内收口，不无限 connecting。

## 证据

tmux 复现（fixture）：

```text
mcp:       connecting 0/1
MCP · configured 1 · connected 1 · armed 1
→ fixture  connected  armed  tools=2
mcp pending (see /mcp)
```

首个 turn 已结束，`/mcp` 已 armed，但 header 与 cue 仍是 pending。挂死 server 配置 `connect_timeout_ms: 2000`，40s 后仍 connecting。

## 根因假设

- TUI 只在 turn 前刷新 loaded-resources；freeze / arm 后没有刷新，cue 仍读旧 `connecting_label`。
- `mcp_tools_pending` 先看 `mcp_connecting_label`，旧 label 会让 cue 继续为真。
- `join_session_prepare` 的 gate timeout 路径没有把 snapshot 收口为 complete / frozen。
- stdio initialize 可能不在 `connect_timeout_ms` 内完成收口。

根因未完全确认；实现前用日志与单测确认每条路径。

## 决策

### 1. 刷新时机

- tool freeze、MCP arm、gate timeout、reload、resume、切会话后刷新 loaded-resources 并重算 cue。
- 不得只在 turn 前刷新；不得依赖用户打开 `/mcp` 才更新 header。

### 2. cue 谓词

- `mcp_tools_pending` 以 settled 状态为准：bootstrap complete 且 tools frozen 时，即使 `connecting_label` 残留也不得显示 cue。
- `connecting_label` 在 connected / armed / timeout 后清除；不得作为唯一 pending 依据。

### 3. gate timeout 收口

- timeout 时生成 settled snapshot：bootstrap complete、tools frozen（armed subset）、诊断保留。
- 清 cue；`/mcp` 显示 timeout 诊断与已武装子集。
- 不自动重试；用户 `/reload` 才重试。

### 4. connect timeout

- `connect_timeout_ms` 覆盖 stdio initialize / handshake；超时收口为 Failed / timed-out。
- 不得无限 connecting；不得让 gate 永 Running。

### 5. 一致性

- header、status cue、`/mcp` 面板消费同一 `LoadedResourcesSnapshot` 或同一派生视图。
- 任一状态变化触发一次刷新；不得各自缓存不同副本。

## 数据流

```text
MCP hub / prepare
  -> SessionPrepareView (connecting / complete / armed)
  -> LoadedResourcesSnapshot (mcp_bootstrap_complete, tools_table_frozen, servers)
  -> TUI root refresh_mcp_short_cue
  -> header + status cue + /mcp panel
```

## Seams 与测试

- harness：pending → armed → cue 清除；gate timeout → cue 清除；hanging server → Failed。
- BDD：reload / resume 后不粘旧 cue；header 与面板一致。
- tmux e2e：真实 TUI 首 turn 后无 pending cue；挂死 server 超时收口。

## 风险

- 过早清 cue 会掩盖真实 connecting；以 bootstrap complete + frozen 为界。
- reload / resume 的 re-gating 仍应显示 cue；只在 settled 后清。
- 连接超时与 gate timeout 语义不同；分别收口，不混为一个状态。

## 迁移

- 不改变 MCP 协议、工具命名或 trust 语义。
- 不新增自动重试。
