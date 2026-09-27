# specs-compact 压缩计划（供审 · v1）

> 临时交接文档（`*.tmp.md` 约定），审批后升格为 change `c2826-refactor-specs-compact` 的 proposal 素材。
> 目标：压降裸规则并补可执行场景，**规范行为不变**。

## 0. 基线快照（2026-09-27，main @ 25dc2127，工作树干净，llman-sdd 0.5.0）

- 64 capability；`validate --specs --strict` 64/64 全绿；review pending 总量 **596**（warningCount 同值）。
- 重点 11 能力 pending 合计 **251**（占 42%）。
- `project dedupe-req-ids --dry-run`：无跨能力 req id 冲突。
- **手工核对发现**：`package-ai-bridge` 文件内 `@req:r1555` 重复挂载两条规则（L29 `thinking-level-resolve-exact`、L110 `deepseek-prompt-cache-read-mapped`），dedupe 工具漏报（只查跨文件）。全局唯一重复。修复：后者换号 **r1902**（当前全局最大 1901）。
- 归档 freeze：archive 共 16 项 / 2.7M，均为近一月条目，不构成评审噪声 → **本轮不冻结**（如需可 `--keep-recent 5` 冻结 10 项，请批示）。
- 活跃 change：c1935（5/30 任务，未绑分支）不阻塞 specs validate；c2470/c2515/c2520 为纯 proposal 草案。c1935 意向落在 infra-otel / provider-trace——本轮对该能力只做场景化不改行为文本，不与其冲突。

## 1. 决策判据（四类）

| 决策 | 判据 |
|---|---|
| **转场景** | GWT 可表达且可用既有 harness/词表（或小额新 step）断言：每条新增嵌套 `场景:`（英文 id）+ `tests/bdd/bindings_*.rs` 绑定 + cargo test 绿。遵循仓库既有约定（451/452 场景有绑定） |
| **保留** | (a) 规则自带「由单测/文档覆盖，MUST NOT 单独扩 BDD step」（平版禁令；限「静态存在性」的弱版禁令按 r1480 先例允许行为场景）；(b) 纯结构/seam/开闭约束（r1535 明确禁 arch guard 元测试）；(c) 真 PTY 依赖且已有专门 PTY req 承载；(d) 负面闭集/无界禁令，无可测触发面 |
| **合并** | 语义等价且承载 req 文本完整包含被并内容（不足处补一句），保留/换号列映射；跨能力合并仅限互为复述且叙事损失可补者 |
| **移除** | 本轮 **0 条**：所有删除均走合并映射，无未替代删除 |

标题稳定性：转场景与合并不改承载 req 标题。执行中发现转场景成本失控（需重基建）时降级为保留并记录理由，不静默跳过。

## 2. 逐能力决策表（重点 11）

图例：**T**=转场景（附场景要点）；**K**=保留（附理由码 §1.a-d）；**M**=合并（→承载 req）。

### app-tui-host（38 → 8）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1240 host-driven-engine | K(b) | 结构不变量；行为面即 r1281 合成切片场景族，另写即重复 |
| r1248 terminal-restore | K(c) | panic/退出恢复需真 PTY；r1282/r1258 两条 PTY 冒烟 req 已承载 |
| r1259 infra-logging-default | T | debug 默认开文件日志 / release 默认关（配置默认态断言，新 step 小） |
| r1270 resize-and-min-size | T | 注入极端尺寸→友好提示非 panic；resize 后重绘（harness 可注入终端，r1277） |
| r1277 host-harness-testable | K(b) | 可测性 meta 要求，场景自指涉 |
| r1278 shared-effect-pump | K(b) | 单一事件泵 seam 结构；行为后果由 r1281 族与 ati headless 场景承载 |
| r1279 single-mux-loop | T | bang 中 Esc 可达 abort + agent 流仍被 poll（复用 bang/steer 词表）；「不逐事件全屏重绘」断言归 r1253 |
| r1280 abort-drops-stream-events | T | 迟到 Xy 不复活 busy（同 ati31 场景族，host 侧臂装时点） |
| r1241 agents-layout-map | K(b) | AGENTS 文档内容要求 |
| r1242 abort-suppress-before-drain | **M→r1280** | 同一同步步进抑制语义两处表述；r1280 文本补 bang Esc 无 suppress 句后删 |
| r1243 host-module-boundaries | K(b) | 组件不直呼 Driver 的结构纪律 |
| r1244 session-read-errors-surfaced | T | 注入 GetMessages 失败→system/error note 而非空 transcript（新 step：注入读失败） |
| r1245 session-list-seam | K(b) | 主体为 seam 禁令；可测片段由 Resume 面板场景族承载 |
| r1246 session-lifecycle-seam | T | set_session_name CR/LF→空格+trim 断言（seam 禁令留文本） |
| r1247 session-resume-manage-seam | T | 删除活跃会话被拒且不调 DeleteSession；确认前不删盘 |
| r1249 reload-orchestrates-foundations | T | 注入单步失败→部分成功继续（扩展 reload 词表） |
| r1250 copy-last-via-driver-seam | T | /history-copy-last 取尾前首条非空 assistant 正文（fake 剪贴板断言） |
| r1251 thinking-level-silent-commit | T | /model 提交成功→仅固定区变化、scrollback 无确认块；**吸收 r1225 的 theme** |
| r1252 loaded-resources-via-driver | T | MCP connecting 时打字/提交/bang/滚历史不被拒（headless 注入 connecting 快照） |
| r1253 tick-gated-local-paint | T | 纯 status 动画帧→上区行缓存命中（渲染计数上界，新 then step） |
| r1254 scrollback-entry-paint-cache | T | 单次 TextDelta→仅变化条目重绘（miss 计数上界） |
| r1255 streaming-assistant-incremental-paint | T | 后缀增长流式→全量解析计数上界+行一致 |
| r1256 mcp-discovery-surface | T | connecting 短 cue 文案 `mcp pending (see /mcp)` 右对齐、收口后收起 |
| r1257 reload-in-progress-ux | T | reload 中 Enter 拒+通知条 body `reloading — wait`；取消/失败收口文案（与 ati43/atc25 共享断言点，host 视角） |
| r1281 synthetic-harness-one-round | T | 主链场景：提交→流式/工具→steer/follow-up→abort→再提交→/exit finish |
| r1282 product-pty-fake-smoke | K(c) | #[ignore] PTY 用例存在性要求 |
| r1258 mouse-input-opt-in-no-moved-paint | T | headless 合成 Mouse Moved→无 render 请求；PTY 断言部分留文本 |
| r1260 interaction-mode-application-owned-default | T | 启动构造绑定 ApplicationOwned+dock 登记；热切禁令留文本 |
| r1261 mode-b-copy-notice-fixed-zone | T | copy-notice→固定区单行提醒，tick 推进 TTL 消失 |
| r1262 enter-follows-transcript-bottom | T | Ready Enter→transcript 滚底+尾插恢复；空输入不 submit |
| r1263 fold-triangle-hit-priority | T | 合成 Left Down 命中三角列→吞按下+toggle（文本自带 harness 验证要求） |
| r1264 attach-tick-no-blocking-rpc | T | 慢 unary 下 spinner 换帧+键入可达（mock HostClient） |
| r1265 attach-mux-session-lifetime | T | WS 断开 last_seq 续订；resync_required 重建 transcript |
| r1266 attach-restore-projection | T | 快照一次重建、无假 spinner、冷磁带不渲染 |
| r1267 attach-reload-cooperative-cancel | T | reload 中取消→经 Host 转达、界面已取消收尾 |
| r1268 attach-fixed-zone-downlink-driven | T | session/resources 帧到达置脏→tick 刷新；无周期轮询（unary 计数） |
| r1269 unary-request-bounded | T | 慢 unary→超时错误呈现非挂起（mock 时钟） |
| r1273 attach-reconnect-grace-ux | T | 宽限内零输出；超宽限通知条；重连窗无 transcript 错误行 |

### layer-architecture（29 → 26）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1520/r1525/r1531/r1533/r1534/r1535/r1521/r1522/r1523/r1524/r1526/r1527/r1517/r1519/r1518/r1516/r1528/r1529/r1530/r1507/r1508/r1509/r12 | K(b) | 分层/seam/开闭/文档卫生结构约束；r1535 自我声明保障方式=AGENTS+review+缝行为测，禁 arch guard 元测试（22 条） |
| r1510 双角色 / r1511 归属判据 | K(d) | 概念角色划分，无可测触发面 |
| r1512 embed 不要求监听器 | T | 无监听环境跑 print 一轮→正常完成（fake provider） |
| r1513 session 原子与分面 | K(d) | 概念划分；行为由 session specs 场景族承载 |
| r1514 一 session 一写者 | T | 第二 attach→只读恢复；只读面写入→失败并说明（server 词表） |
| r1515 产品 TUI 要求监听器 | **M→cli r1391** | 互为复述；r1391 含产品细节（非零退出+提示 serve+print 豁免）；r1512 文本补指针句 |

### app-tui-fixed-zone（26 → 2）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1214 idle-status-omitted | T | idle status 0 行；busy 至多 1 行且不进 footer |
| r1224 footer-minimal | T | footer 1 行、字段序、thinking 省略、无队列徽章、右截断（**吸收 r1218**） |
| r1218 footer-token-provenance | **M→r1224** | r1224 第二句已逐字复述 provenance 文案规则 |
| r1226 footer-derived-context-percent | T | ` · 32.8%/128k` / `~p%/W` / `?%/W` 格式与禁作触发 SSOT |
| r1229 footer-used-compact-count | T | k/M 紧凑阈值表；`?` 不紧凑；保 tokens 词 |
| r1233 theme-glyphs-config | T | glyph 档显式配置切换渲染差异 |
| r1234 design-docs-ssot | K(b) | 文档 SSOT 存在性要求 |
| r1235 demo-theme-auto-detect | T | agent_demo 环境变量切 Light/Dark+theme_mode 暴露 |
| r1236 idle-editor-compact | T | 空草稿操作区紧凑行数 |
| r1237 busy-status-spinner | T | spinner+短词同组合贴左 |
| r1238 user-message-no-wash | T | 用户行无淡底、保留前缀、无 status 轨 |
| r1239 compaction-retry-single-status-line | T | Compacting/Retry 态 status 仍 1 行 |
| r1215 abort-status-idle | T | abort 后 status 回 0 行+滚动提示一行 |
| r1216 models-picker-footer | T | idle footer 收敛 active；busy 立即反映 selected（run 绑定） |
| r1217 status-spacer-above-editor | T | busy 前导空行+短词紧贴 editor；idle 恰一行空白无 Ready |
| r1219 footer-token-refresh | T | settlement 后刷新；同轮不二次 estimate（计数断言） |
| r1220 主题热应用 | T | 按名应用；未知名保留旧主题+诊断 |
| r1221 theme-slash-slot | T | /theme 开槽、选中应用、Esc 不改 |
| r1222 thinking-border-and-footer | T | level→边框染色+footer 标签；bash 边框覆盖/恢复 |
| r1223 loaded-resources-slot | T | 头卡渲染：skills/mcp 行、connecting 进度、失败单行诊断（可拆 2 场景） |
| r1225 fixed-zone-success-no-system | **M→ath r1251** | 同一行为两面；r1251 文本 model/thinking 扩含 theme |
| r1227 toast-notice-ephemeral | T | 单槽替换、`Error: ` 前缀、TTL 清除 |
| r1228 fixed-zone-footprint-term-budget | T | 短终端高槽 body=预算≥1；busy 时 status lead 可见 |
| r1230 reload-status-and-toasts | T | reload 态 Loader 形态+三态通知条文案 |
| r1231 fixed-zone-no-extra-undocumented | K(d) | 负面清单无界（「仅 SSOT 已声明槽位」） |
| r1232 tool-header-timeout-note | T | header 显示 `(timeout Ns)`、省略时不显示 |

### app-tui-input（24 → 1）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1283 editor-operation-zone | T | 选择器替换 editor 槽而非画内容顶部 |
| r1314 yolo-after-trust | K(d) | 负面禁令（不实现审批流）；信任后直执已由 tools 场景覆盖 |
| r1321 double-esc-session-tree | T | 空编辑器双 Esc 开树；树开 Esc 关树 |
| r1322 demo-steer-preserves-turn | T | demo steer 写 transcript 不顶替忙碌轮 |
| r1323 demo-bash-prefix-border | T | demo `!` 前缀边框强调色 |
| r1324 demo-external-editor-stub | T | demo Ctrl+G 记录调用不 spawn |
| r1285 product-queue-fixed-zone | T | 队列条 Steering:/Follow-up:+Alt+Up 还原；无 footer 徽章 |
| r1286 product-queue-uplink | T | 注入后 transcript 留用户气泡（**吸收 r1320**） |
| r1320 same-text-second-user-visible | **M→r1286** | r1286 末句逐字包含其全部语义 |
| r1287 product-editor-send-history | T | 发送后 ↑ 召回；空串/连续重复跳过 |
| r1316 editor-history-session-seed | T | 新会话播种最近 N session；resume 替换缓冲 |
| r1289 product-bash-prefix-border | T | `!`/`!!` 边框切换；去前缀恢复 thinking 边框 |
| r1291 product-external-editor | T | harness stub 路径写回；未配置→Error 短行保留原文本 |
| r1292 editor-slot-machine-live-tree | T | 槽替换贴底、Esc 优先关槽（挑槽位断言） |
| r1296 model-picker-slot | T | 无参 /model 开槽、Enter 提交 (model,level)、Esc 不改（与 atm r1188 分层互补） |
| r1298 model-arg-completion | T | `/model ` 补全选定写入；Esc 关 popup 不 SetModel |
| r1304 session-resume-panel-keys | T | Tab/Ctrl+S/Ctrl+N/Ctrl+R/Ctrl+D 键位族（可拆 2 场景） |
| r1309 at-path-completion | T | `@` 弹文件补全；Esc 不改非补全文本 |
| r1310 paste-collapse-submit | T | 提交文本为展开正文而非 paste marker |
| r1311 keybindings-catalog-hot-reload | T | 加载覆盖；失败保留旧绑定+诊断 |
| r1315 dollar-skill 补全与高亮 | T | `$` 弹补全；scrollback 高亮 $name |
| r1313 paste-image-temp-path | T | 剪贴板图→tempfile 绝对路径文本；皆无→Error 短行 |
| r1317 abort-keeps-partial | T | 流中 abort→partial flush 正式条目+abort 脚注 |
| r1319 attach-queue-stats-calibrate | T | Host 真实深度 FIFO 消退；空深度不清已画条 |

### package-ai-bridge（22 → ≈19，另修换号）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1536/r1546/r1557/r1559/r1560/r1543 | K(b) | 包边界/模块划分/DTO 独立/映射 seam/实现归属/词汇 SSOT（结构约束） |
| r1537 vendor-sdk-first | K(b) | 接线策略与开闭原则 |
| r1538 thinking-params-in-request-body | T | 请求体 thinking 字段按 compat 分族（组装纯函数 golden 断言） |
| r1555 thinking-level-resolve-exact | K(a) | 自带「MUST NOT 为大小写拒绝单独扩 BDD step」；规则主体即精确匹配 |
| r1556 provider-http-bounded | T | 慢 chunk 流→chunk-gap 超时分类错误（mock SSE） |
| r1544 responses-error-message-surface | T | 嵌入 JSON 错误串→message 暴露（纯映射断言） |
| r1545/r1547/r1548/r1549/r1550/r1551/r1552/r1553/r1554 | K(a) | 各自带平版「MUST NOT 单独扩 BDD step」（默认板/范围/三档/观测诚实/assembler 唯一/全量回放/回填/幂等） |
| r1555(第二个，deepseek-prompt-cache-read-mapped) | 换号→**r1902** | 修复文件内重复 id；决策 K(a)（自带平版禁令） |

> 该能力大量规则自我声明由包内单测承载，压降空间天然有限——这是诚实结论，不为凑数强转。

### cli-entry（21 → 5）

| req | 决策 | 要点/理由 |
|---|---|---|
| r67 info-fields / r68 source-enum | K(b) | 数据字段/枚举封闭性（静态契约） |
| r69 builtin-table | T | GetCommands 含 session-* 不含 pi 短名（同 as r1063 场景型，cli 侧断言） |
| r70 dispatch | T | idle slash 经 dispatch 断言（**吸收 r1380**） |
| r1380 dispatch-via-shared | **M→r70** | 与 r70 几乎同句（另 atc r1198 为第三处同义，保留 r1198 为命令面 SSOT） |
| r1379 app-directory / r1392 composition-root-ports / r1394 assembly-via-bootstrap | K(b) | 目录/装配结构约束 |
| r1389 no-silent-fallback | T | 零显式模型配置→bootstrap 硬失败指向配置 |
| r1393 serve-subcommand | T | `serve --port 0` 打印实际端口；stop/install 子命令 |
| r1381 default-tui-entry | T | TTY 无子命令→默认 TUI（解析结果断言） |
| r1382 print-requires-prompt | T | print 无 prompt→错误，非 Hello! 占位 |
| r1378 session-tree-kind-api | T | 未实现 kind→明确错误 |
| r1383 stable-session-id-on-run | T | 两轮 run 同 session 文件不新建 |
| r1384 tokenizer-cache-subcommands | T | status/download/clean 早退不装配会话（steps_tokenizer 词表现成） |
| r1385 cli-surface-vs-ops | T | 顶层 --tui/-p 解析失败；管理动词保持顶层 |
| r1386 config-load-fail-closed | T | 坏 YAML→非零退出 |
| r1387 unset-model-display | T | 未选模型→NOT-SET 展示 |
| r1388 surface-owned-flags | T | print 拒 --list-models/--attach/--port；--attach 与 --port 优先级 |
| r1390 resume-hint-on-exit | T | 退出 stderr 恰一行 Resume 提示；未持久化不打印 |
| r1391 tui-attach-fail-closed | T | 未在听→非零退出提示 serve（**吸收 layer r1515**） |

### infra-otel（20 → 6）

> 该能力 headless 模板现成（`假如 mock 模型先 tool 后无 tool / 当 以观测闸开启…`，steps_otel_obs）；r1480 先例证明带「静态存在性」限版禁令的规则也可挂行为场景。

| req | 决策 | 要点/理由 |
|---|---|---|
| r1466 otel-default-none | T | 未配置→零 OTLP 出口 |
| r1477 otel-config-opt-in | T | 合法配置→reporter 安装 |
| r1486 otel-fallback-no-block | T | 坏 endpoint→主路径不失败+obs_diag 单行 |
| r1487 otel-fastrace-only | K(b) | 依赖卫生禁令（禁引入 tracing crate） |
| r1488 otel-fanout-with-file | T | 双开→同批投递两侧 |
| r1492 otel-generation-usage | T | 带 usage Done→usage_details；无 usage 不伪造 |
| r1470 otel-token-estimate-parent | T | 闲置路径独立根+session id；同 settlement 至多一个 |
| r1471 otel-tool-observation-io-tier | T | truncated→tool.execute 带摘要；none→不带 |
| r1472 otel-turn-input-preview | T | truncated→agent.turn input 提示摘要 |
| r1473 otel-generation-request-body-input | T | request JSON 为 input 源 |
| r1474 otel-generation-abort-finalize | T | abort→ERROR+aborted status，不伪造 usage |
| r1475/r1476/r1478/r1479 | T | 限版禁令（「静态存在性」）；行为断言非静态存在性（r1480 先例）：并行 span 树 / compaction span / turn 终态 / settlement-once |
| r1481/r1482/r1483/r1484/r1485 | K(a) | 平版「MUST NOT 单独扩 BDD step」（会话身份/槽写纪律/双身份 fork 边/skipped span） |

### agent-runtime（20 → 8）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1039 system-via-generate-options | T | 首轮 history 首条为真实 user 输入、无 system 行（限版禁令） |
| r1055 builder-ports / r1056 sessionstore-eventsink | K(b) | 装配/端口存在性 |
| r1057 mutable-slots-next-turn | T | run 中 set_tools 下轮生效（两轮 mock 断言） |
| r1031 unknown-event-degrade | K(a) | 指定 protocol 单测载体 |
| r1032 defaults | K(a) | 指定单测载体 |
| r1033 cwd-validate | **M→store r1094** | 加载侧已有场景；r1094 文本补「创建/导入同样校验」 |
| r1034 wire-event-mapping | **M→protocol-app r1698** | 映射往返已被 r1698 场景承载；r1698 文本补映射要求句 |
| r1035 Driver context 重载缝 | T | 信任下重载应用；未信任不注入 |
| r1037 Driver skills 重载缝 | T | 同上（skills）+ 查询已加载名 |
| r1040 persist-done-usage | T | 文本自带「MUST 有可执行场景」：assistant 持久化含 usage/stop_reason |
| r1042 next-turn-refresh-model-thinking | T | 文本同上：turn 边界重读 model/thinking；in-flight 不拆流 |
| r1043 abort-persist-skip-llm | T | abort partial 落盘；投影跳过 aborted/error 行 |
| r1049 turn-end-threshold-compaction | T | Settle 后 threshold 预检（domain-compaction 词表） |
| r1050 turn-end-overflow-compact-retry | T | overflow→compact-and-retry 续跑（锚点已在 domain-compaction） |
| r1051/r1052/r1053/r1054 | K(a) | 平版禁令（ContextPolicy / MCP 冻表 / error kind / streamTiming） |

### protocol-app（17 → 4）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1693 Command 枚举 / r1695 Event 枚举 | K(b) | 枚举闭集存在性声明 |
| r1699 会话命令分发 | T | SwitchSession 经 store 校验；GetMessages 返回条目（server 词表） |
| r1692 dispatch 队列命令 | T | steer/follow_up/clear_queue 路由 Driver（r1691 为解析侧，此为路由侧） |
| r1694 线协议不镜像厂商事件 / r1705 闭集缺口禁旁路 | K(d) | 负面闭集/时态性禁令 |
| r1719 线协议映射队列与闭集 | T | QueueUpdate wire 往返+未映射降级不 panic |
| r1711 agent-part-tagged-wire | T | JSONL content type 判别形态（steps_protocol） |
| r1712 preview-text-excludes-thinking | T | message_text 只聚合 text |
| r1707 xy-event-error-kind | T | kind 结构+往返保真 |
| r1708 error-kind-at-source | T | 失败注入→稳定 kind（缺/IO/校验/不支持） |
| r1718 todo-updated-wire | T | TodoUpdated wire 表达+不进冷回放 |
| r1714 会话能力方法表 | T | 树/travel/label/list/entry/新建/名称读写删批量登记断言 |
| r1715 Host 资源方法 | T | reload+loaded_resources 真实连接态（禁假完成） |
| r1721 CompactionEnd 载荷下行 | T | 全载荷往返保真+旧形态缺省解码 |
| r1717 上下文估计方法 | T | estimate_context 与本地同源 |
| r1703 bang 与工具分流 | T | bash 直播走独立事件路径不并工具流 |

### app-tui-commands（17 → 2）

| req | 决策 | 要点/理由 |
|---|---|---|
| r1188 slash-exit-model | T | /model 无参开槽/有参直设/未知斜杠错误（**吸收 r1200**） |
| r1200 idle-slash-submit | **M→r1188** | 与 r1188 同义；增量「系统错误行」措辞并入 |
| r1198 dispatch-shared | T | slash 经共享 dispatch 断言（同为 cli r70/r1380 合并的承载 SSOT） |
| r1199 extensible-commands | K(d) | 扩展性元属性 |
| r1201 debug-scene-slash | T | debug 构建 /debug 列场景/隔离会话注入（cfg(debug) 绑定） |
| r1202 slash-session-tree-fork | T | /session-tree 开树、/session-fork、busy 拒 |
| r1203 commands-module-growth | K(b) | 模块收口结构约束 |
| r1204 slash-session-io | T | compact 无参/带参、export 后缀分派、import 确认（可拆 2 场景） |
| r1205 slash-session-info | T | 无参 /session 信息转储 |
| r1189 slash-session-resume | T | 面板打开；busy switch 拒+通知条文案（atm10 常量） |
| r1190 slash-session-lifecycle | T | new/clone/name 规范化（可拆 2 场景） |
| r1191 slash-reload | T | idle /reload 热重载、busy 拒 |
| r1192 slash-trust | T | 决策入 store+提示需 reload；未配置拒写盘 |
| r1193 slash-history-copy-last | T | 复制最近 assistant 正文；无则提示 |
| r1194 slash-theme | T | 无参开槽/有参应用/toggle |
| r1195 busy-slash-policy | T | Allow/Reject 分流（bang 中 /model Allow、/reload 拒） |
| r1196 slash-mcp | T | /mcp 缓存快照同步挂载、Esc 关槽不 abort |

### agent-session-store（17 → 4）

| req | 决策 | 要点/理由 |
|---|---|---|
| r31 快照操作 | K(b) | 7 操作存在性总则（历史遗留词汇），逐操作断言成本不成比例 |
| r41 compaction | K(d) | store 侧总则；行为细节由 domain-compaction 与 ar r1049 场景承载 |
| r1106 禁止旧 AgentPart 迁移 | T | 植入旧 untagged 行→load 跳过可观测（r1098 同型步骤） |
| r1109 写入安全 | T | 植入中途状态目录→旧 manifest 可恢复；orphan 不入会话 |
| r1101 fork-header-cut-entry | T | 子会话头父 id+切点；旧文件缺字段不失败 |
| r1102 v7 manifest 提交 | T | orphan/临时段不影响 load/list；sealed 不改写 |
| r1103 冷段按需恢复 | T | 多 cold 段 resume 不全量解析（IO 计数） |
| r1900 sealed-sidecar-index | T | sidecar 先筛；损坏回退不 false negative |
| r1901 resume-projection-llm-api | T | leaf 上 modelChange/thinkingLevelChange 投影不钳制 |
| r1095 session-entry-camelcase-v7 | T | header.version==7、外壳 camelCase、bang 写 type=message（补 r1097 未断言部分） |
| r1098 session-load-skip-warn | T | 坏行跳过+warn ≤3 收敛 |
| r1113 CWD 呈现 | **M→r1112** | r1112 场景已断言文案含两路径；r1112 文本补「面向用户错误即该文本」 |
| r1114 CWD 集成 | T | CLI 与 RPC 恢复前同一校验 |
| r1115 SessionStore 端口实现 / r1116 ExportIo 实现 | K(b) | 端口实现/注入结构（与 as r1077 互补不合并） |
| r1117 日志访问 | T | read_recent 供 journal（server 词表） |
| r1089 分享指引 | T | 未配置 token→配置指引错误 |

## 3. 汇总

| 能力 | pending 前 | 转 | 并 | 留 | pending 后 |
|---|---|---|---|---|---|
| app-tui-host | 38 | 29 | 1 | 8 | 8 |
| layer-architecture | 29 | 2 | 1 | 26 | 26 |
| app-tui-fixed-zone | 26 | 22 | 2 | 2 | 2 |
| app-tui-input | 24 | 21 | 1 | 1 | ~1 |
| package-ai-bridge | 22 | 3 | 0 | 18(+换号) | ~19 |
| cli-entry | 21 | 15 | 1 | 5 | 5 |
| infra-otel | 20 | 14 | 0 | 6 | 6 |
| agent-runtime | 20 | 10 | 2 | 8 | 8 |
| protocol-app | 17 | 13 | 0 | 4 | 4 |
| app-tui-commands | 17 | 14 | 1 | 2 | 2 |
| agent-session-store | 17 | 12 | 1 | 4 | 4 |
| **合计** | **251** | **155** | **10** | **84** | **~85** |

- 仓库总 pending 预期 596 → **~430**（非重点 53 能力本轮不动，可作第二波）。
- 合并映射共 10 条（含 4 条跨能力：r1225→ath r1251、r1515→cli r1391、r1033→store r1094、r1034→protocol-app r1698），移除 0 条；换号修复 1 条（r1555 重复→r1902）。
- 承载 req 文本需补句的 5 处：r1280（bang Esc 无 suppress）、r1251（+theme）、r70（+不复制第二套）、r1094（+创建/导入校验）、r1698（+映射要求）。

## 4. 执行与验证（审批后）

1. `llman-sdd change new c2826-refactor-specs-compact` → proposal/tasks（本文件为素材）→ `change start` 绑分支。
2. 逐 capability 一个 commit（specs .feature + 对应 bindings/steps），边改边 `validate <cap> --strict` + `cargo test`。
3. r1555→r1902 换号单独小 commit。
4. 全量门禁：`llman-sdd validate --specs --strict` 全绿；`llman-sdd review` pending 与本表一致；`just qa`（fmt/lint/test 全套）。
5. 每类错误最多 3 轮自修，不绿即停并贴输出。
6. `change finalize`（squash）收口，附本计划与映射表。
