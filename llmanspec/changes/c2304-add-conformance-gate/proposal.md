---
depends_on:
- c2300-update-cs-capability-split
- c2301-update-stable-wire-protocol
- c2290-update-standalone-host
- c2302-update-host-multi-session
- c2303-update-unified-entry
branch: sdd/c2304-add-conformance-gate
base_sha: 36b70adcf6608ddc9a2614599d494758badc6f7d
checkpointed: false
---

# 产品位：符合性闸

同一张方法表（c2290 为底，本票扩 TUI 已用的会话/reload/MCP 行；c2301 闭集作 Event 载荷；四象限作信封），对 `InProcessClient` 与 `HttpWsClient`（测试 host + 真 POST/WS）各跑一遍。未实现已承诺方法、或双 carrier 漂移 → 红。落地 `test-*`。

切片：

1. **已承诺 unary 双跑**（EchoHost 换成真 Host + `writerToken`）。
2. **工具 chrome 对拍**（attach 不得 `Read ...`）。调研：`research/tool-chrome-wire-gap.md`。
3. **能力对齐**：把产品 TUI 已经调用、Remote 仍 unsupported/空快照的缝登记进方法表并接到 Host。调研：`research/capability-align.md`。

对齐完成后再做方案 A（c2315 依赖本票）。

## Why

没有同表双跑，embed 与 attach 会再分叉。方法表绿了但 Event 载荷被削掉，工具行仍是占位 `...`。会话树 / resume / `/reload` / MCP 头卡在 attach 上直接 unsupported 或空快照，两终端能聊也不能当产品 TUI。

## What Changes

- 场景覆盖方法表（含拒绝 / 缺 session / 写者冲突 / 未知方法）。新协议语义必须带场景。
- 一侧 `InProcessClient`，一侧 `HttpWsClient` + 真序列化与传输（POST + WS 下行），不 mock 掉协议。c2290 的 `InProcessClient` 现为 EchoHost 座位，**不是**真 dispatch；本票 MUST 换成与 Host 同表的 in-process 实现，禁止 echo 过闸。接上真 HostState 时 MUST 与 `HttpWsClient` 一样持有 `writerToken`。
- 面本地（`Quit`、键、画、剪贴板）不进表。
- 只读 / 写者冲突语义入表。
- **下行 Event 载荷对拍（先红后绿）**：同一条带 path 的工具 Start，进程内 `XyEvent` 与经 wire 往返（及双 carrier 订阅）喂进同一套 TUI 投影后，人类可读 `args_preview` MUST 相等且 MUST 含路径。现有 activity_fold harness 直接注入完整 `XyEvent`，**不算**本条。实现顺序 MUST 先落失败对拍，再给闭集内 `ToolStart` 补 `args`（缺省反序列化兼容）；流式意图 chrome 若仍饿死，同切片让 `MessageUpdate` 带上 toolCall。**禁止**新开 Event 变体或双解析旧无 args 形状。
- **扩方法表并接线（TUI 已用缝）**：至少登记并双跑 `session_tree`、`travel_session_tree`、`append_entry_label`、`list_sessions`、`load_session_entries`、`new_session`、`get_session_name`、`set_session_name`、`set_session_name_for`、`delete_session`、`reload`（进程级 skills/MCP/prompt 重装，不是面本地键位/主题）、`loaded_resources`（MCP/skills 只读快照）。`leaf_entry_id` MUST 可经已有 `get_state` 或一条只读 unary 取得，禁止 Remote 恒 `None` 导致 fork 面板空转。`XyRemoteDriver` MUST 走这些 unary，MUST NOT 再 `unsupported` / 默默返回空快照 / reload no-op。live `sr-st1` 改为「经已登记 unary，禁止旧 REST」。specta 重生。c2302「本票不扩表」对本票不适用。

## 非目标

不开新信封、不换载体、不新开 Event 变体、不把全量 `XyEvent` 1:1 上线。不是真 PTY / tmux E2E。方案 A（一条命令 loopback）**不在本票**（c2315，等本票归档）。`load_debug_scene`、`persist_project_trust`、bash 直播增量、`queue_stats` 仍保留。键位/主题热加载仍面本地（`/reload` 的 client 半边）。
