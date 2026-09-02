# Design：c2480 Attach 连接韧性状态机

## D1 握手统一：mux 首帧为唯一握手（用户拍板）

- `RpcMessage` 新增 `ServerHello { protocol: u32 }`（与 `PROTOCOL_VERSION` 同型，
  serde tag `server_hello`）；server 侧 events.mux 连接建立后**首帧**发送；客户端
  每条连接校验，不符即 fatal（推错误事件、终止该代循环），与今日 describe 不符
  语义一致——版本错不是可重试故障。
- driver 下行主路径退役 `host.describe`：`ensure_downlink` 的握手步骤与
  `attach_session` 的预检都改为依赖首帧 hello（subscribe 前置校验天然由 mux 序给
  出）；`handshake_done` 进程级闩存随之删除。`host.describe` 保留注册（外部探针、
  BDD steps_server 既有用途），不再是 attach 关键路径。
- 与 protocol-app「MUST NOT 同时维护两套版本协商」的关系：hello 携带的就是
  `PROTOCOL_VERSION` 单一语义源；describe 不再参与协商，两处检查收敛为一处。
- 附带修复：今日 `handshake_done` 只在进程首连验一次，Host 重启换协议版本后
  attach 不重验；每连接 hello 让版本漂移在重连瞬间以 fatal 暴露。
- DTO 处置：`ServerFrame::ServerHello`（内部 DTO、从未上线）随 wire 化删除；
  `ServerFrame`/`ClientFrame` 其余变体的死码盘查不在本票。

## D2 状态机落点：driver 层补齐，不做 carrier 下移

research 笔记（2026-08-23）原判断「状态机落 host_client 层」，但其时 driver 的
`ensure_downlink` 重连循环尚未被纳入对照。现状：循环、退避、ping 判死、last_seq
续传、resync 全在 `src/app/core/driver/remote.rs` 且工作正常。本票**不做**整体
下沉（纯组织搬动违反 pre-0.0.1 卫生），按缺口分置：

- 首帧 hello 的帧级校验在 carrier（`http_ws.rs` mux 流：首帧必须可解析为
  `ServerHello`，否则按连接失败交还 driver 判定）；
- 存活归零退避、generation、fatal 判定在 driver `ensure_downlink`；
- 宽限 UX 在 TUI host 层（chrome 词汇，不新增信息面概念）。

## D3 常量与注入

| 量 | 起步值 | 来源 |
|---|---|---|
| 退避起点 / 封顶 | 200ms / 5s | 沿用现有 ensure_downlink |
| 存活归零阈值 | 1s | 参考实现 reconnectDelay |
| 初始连接宽限 | 5s | 参考实现 startup grace |
| 重连宽限 | 1s | 参考实现 event-stream retry grace |
| 合帧窗口 | 10ms | 参考实现 publish flush |

常量起步、不进配置文件；可执行场景经构造函数/字段注入微秒级短值，使时序断言
毫秒内完成（测试专用注入，不为产品开配置面）。

## D4 测试双缝（用户拍板）

- **mock HostClient 缝**（主）：直驱 `XyRemoteDriver`（remote.rs 既有测试模式），
  覆盖退避升级（闪断连接存活 < 阈值 → 退避逐次翻倍；存活 ≥ 阈值 → 归零）、
  generation（restart 后旧代迟到帧被弃）、hello 校验（版本不符 fatal 不重试）、
  合帧（窗口内 N 事件一次投影）。确定性、毫秒级。
- **真进程缝**（1 条）：`steps_server.rs` 的 `serve()` 风格起真 server + 真
  HTTP/WS：订阅 → 杀 server →（同版本）重启 → 客户端重连续传（last_seq 续放 +
  宽限内无错误行上屏）→ journal 事件兑现。专杀「in-process 绿、默认 attach 路径
  坏」一类静默缺口（sr-method1 教训的 live-call 化）。
- 宽限 UX（ath42）为 TUI 呈现语义，落 TUI harness 单测，不做跨缝 BDD。

## D5 UX 分级（chrome 词汇表约束）

- 断线重连属 **E 类运行态**：落**壳层通告**（status 上方独立一行，warning 色，
  TTL 自动清除），禁止冒充对话条目 / 滚动提示（词汇表 SSOT：
  `docs/architecture/TUI信息面与chrome词汇.md`）。
- 宽限内（初始 5s / 重连 1s）零打扰；超宽限出「连接断开，重连中」通告；恢复成功
  出通告并即时清除断线态；重连窗内 MUST NOT 向 transcript 推错误行（替换现状
  `push error_msg` 刷屏路径）。
