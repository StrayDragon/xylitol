# Design：压缩 specs 重复与重叠规则

## 原则

- 只合并语义等价或同一行为面的规则；主题相近但行为不同的规则保留。
- canonical `@req` id 取最早/最稳定的那条；被删规则的 `场景:` 全部并入 canonical。
- 不改 `功能:`、`背景:`、`规则:` 之外的标题层级；保持 2/4 空格缩进。

## 合并矩阵

| capability | canonical | 并入 | 说明 |
|---|---|---|---|
| agent-session-store | r1094 `会话 CWD 校验` | r1111 / r1112 / r1114 | 加载/创建校验、fallback、错误文案、CLI/RPC 顺序同一行为面 |
| agent-session-store | r1108 `分支摘要` | r1091 | generateBranchSummary 与 fork 时摘要内容同一行为面 |
| agent-session-store | r1110 `会话 fork` | r1092 / r1101 | fork 复制、返回子 id、头字段同一行为面 |
| agent-tools | r42 `默认内置工具闭集` | r1144 | 同一 7 工具闭集；并入 read/write/bash/ls 场景 |
| agent-tools | r32 `工具 trait 前缀` | r1135 / r1143 | XyTool、CancellationToken、XyToolExecutionMode 同一命名/端口面 |
| agent-runtime | r1045 `tool-batch-barrier-parallel` | r1044 | 缺省模式即 barrier_parallel，并入缺省场景 |
| domain-compaction | r1395 `token 估算使用统一 Usage 与多源计量` | r1401 | 同一 canonical XyUsage + provenance 面 |
| domain-compaction | r1408 `overflow 识别` | r1409 / r1410 | 识别、一次 recovery、与 threshold 分流同一 overflow 面 |
| protocol-app | r2918 `错误码与业务码` | r1696 | 信封成功/失败形状与业务码同一错误面 |
| protocol-app | r2919 `方法名寻址与预留命名空间` | server-core r1904 | 方法名寻址、未登记错误、幂等键同一方法面 |
| protocol-app | r1704 `握手与版本` | server-core r1911 | host.describe protocol/formats 协商同一握手面 |

## 不合并（语义互补）

- agent-session r1077 `SessionStore 端口` 与 agent-session-store r1115 `SessionStore 端口实现`：前者约束对话循环不得持有文件系统细节，后者约束端口方法面。
- agent-plan r2951 `plan-mode-prompt` 与 agent-runtime r2955 `plan-mode-middleware`：前者是模型上下文契约，后者是运行时中间件行为（含静默降级）。
- agent-runtime loop-middleware r2920–r2931：逐条为不同 SPI/控制/闸/边界行为，合并会丢语义。
- server-core r1902 `产品线协议 JSON-RPC 信封`：承载面约束（WS text / POST JSON）与 protocol-app 信封形状不同层，保留。

## 验证

- `llman-sdd validate --specs --strict`
- `llman-sdd project dedupe-req-ids --dry-run`
- `just qa`
- `llman-sdd review`（critical 0；unbound 不因合并上升）
