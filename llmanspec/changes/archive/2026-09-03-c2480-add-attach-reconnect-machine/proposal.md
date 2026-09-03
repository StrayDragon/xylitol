---
depends_on: []
branch: sdd/c2480-add-attach-reconnect-machine
base_sha: cf68a0fc1936c3bdab7d232562dc62a3008d4a5f
checkpointed: true
checkpoint_sha: cf68a0fc1936c3bdab7d232562dc62a3008d4a5f
---

# Attach 连接韧性状态机：每连接握手、退避升级、代际失效、宽限 UX 与下行合帧

## Why

attach 是产品 TUI 默认路径，连接断续应作为常态事件设计。本票源自 delayed-changes 的
原提案；**现状已核对到 2026-09-03 代码**，与原提案文本（2026-08-23 research）相比，
重连循环、半开检测（c2425 ping/40s 判死）、last_seq 续传与 resync 重建（c2307/
ath35/ath40）均已落地——「断线即败或静默」不再成立。真实剩余缺口是五个：

1. **握手只验一次**：握手靠 unary `host.describe` 且 `handshake_done` 永久闩存，
   Host 重启换协议版本后 attach 客户端不重验；`ServerFrame::ServerHello` 只是内部
   DTO（ws.rs 文件头注明非产品 wire），生产路径从未发送，「连上了但不是本服务」在
   WS 层不可判。
2. **退避不升级**：每次 WS 连上即把 backoff 归零（remote.rs:284），闪断链（连上
   几百 ms 即死）永远以 200ms 猛敲端口。
3. **无代际失效**：`restart_downlink` 后旧连接迟到帧可推进共享 downlink 队列，
   切会话/新会话窗口内可串台。
4. **断线 UX 无分级**：每次重连失败 `push error_msg` 直接刷进 transcript，恢复
   成功零通告——Host 重启的日常动作换来一屏红字。
5. **客户端无合帧**：高频工具流逐事件投影（host 侧有渲染节流，client 侧无）。

## What Changes

- **mux 首帧唯一握手**（已拍板）：`RpcMessage` 新增 `ServerHello { protocol }`
  变体，server 在 events.mux 连上后首帧发送；客户端**每条连接**校验版本，不符按
  既有 describe 不符语义 fatal（MUST NOT 降级、MUST NOT 重试风暴）。driver 下行
  主路径退役 `host.describe`（保留注册方法供外部探针与 BDD），满足 protocol-app
  「MUST NOT 同时维护两套版本协商」。`ServerFrame::ServerHello` 死 DTO 一并处置。
- **退避升级**：重连循环维持 200ms→5s 指数退避；单次连接存活 ≥ 阈值（起步 1s）
  才归零——闪断按持续故障升级，长活连接才重置。
- **generation 代际失效**：重连/重订循环带代际计数，旧代迟到帧 MUST NOT 进入新代
  投影。
- **宽限分级 UX**：初始连接与重连各给宽限期（起步 5s / 1s，常量注入）；宽限内
  MUST NOT 打扰信息面；超宽限以**壳层通告**（chrome toast，E 类运行态，词表 SSOT
  见 `docs/architecture/TUI信息面与chrome词汇.md`）提示断线重连中，恢复成功通告；
  断线 MUST NOT 再刷 transcript 错误行。
- **下行攒批合帧**：短窗口（起步 10ms）内多条下行事件合并一次 UI 投影。
- **specs landing**：新规则全落 `app-tui-host.feature`（ath41–ath44，attach 族延
  续线，不改既有条款）；可执行场景按双缝落地——mock HostClient 缝为主（退避升级/
  代际/hello 校验/合帧），另加一条真进程缝（spawn serve → 断连 → 重连 → journal
  续传端到端）。

## 非目标

- 不做跨重启会话自动续跑（Host 重启后的续跑策略是 host 侧独立议题）。
- 不引入多 endpoint 自动漂移换血（managed-service 式恢复不在本票）。
- 不做宽限/退避/合帧参数的用户配置化（常量起步，测试经构造注入短值）。
- `ServerFrame`/`ClientFrame` 整体死码盘查不在本票（随 dead-code audit），只处置
  与 hello wire 化直接冲突的 `ServerHello` 变体。

## Impact

- `src/protocol/wire/envelope.rs`（+1 信封变体）；`src/app/server/ws.rs` 及 mux
  路由（首帧发送）；`src/app/core/host_client/http_ws.rs`（首帧校验）；`src/app/
  core/driver/remote.rs`（退避升级、generation、describe 退役、合帧）；`src/app/
  tui/host/`（宽限 + 壳层通告）。
- `llmanspec/specs/app-tui-host/app-tui-host.feature` +4 规则与可执行场景；
  `tests/bdd/` 新增 mock 缝与真线缝步骤/绑定。
- 门禁基线：既有 BDD 全绿基础上按新场景递增；为 gpui（c2350）趟熟连接层公共模式
  （每连接握手、generation、宽限分级、合帧），是其解锁前置。
