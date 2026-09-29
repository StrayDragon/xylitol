# 第二波 specs-compact 压缩计划（供审批）

基线：llman-sdd 0.5.1，validate --specs --strict 64/64 全绿；裸规则 477 条 / 62 capability；
dedupe-req-ids 无碰撞（含 0.5.1 新增同文件碰撞检出）。

## 判据（沿用第一波）
- 转场景：行为可程序化断言——优先复用既有 BDD 步骤（1440 条短语清单比对），其次新步驱动既有 harness；
- 保留（裸规则维持）理由码：(a) 规范自declared「由单测覆盖/MUST NOT 扩 BDD step」或包内纯函数 carrier；
  (b) 行为已被既有场景覆盖（点名场景族）；(c) 缺断言面（需新插桩/harness，点名缺什么）；(d) 结构/流程契约
  （分层依赖、命名、文档 SSOT、just qa 流程——layer-architecture 规范自身禁止 grep 元测试）；
- 合并：语义等价才并，标题取存活规则；移除：须有替代或「零实现且与现行 spec 冲突」证据。
- 「MUST NOT 为静态存在性扩 BDD step」条款只禁存在性检查，不禁行为场景（第一波先例 r1476/r1479）。

## 四类决策总量
| 决策 | 数量 | 说明 |
|---|---|---|
| 转场景 | 91 | 新增约 95 个可执行场景（部分规则 2-3 个） |
| 合并 | 3 | r1758→r1727；r1756→r1841（跨能力）；r1768+r1769→r1767 |
| 保留 | 373 | 逐条带理由码（明细见决策稿） |
| 移除候选 | 10 | 全部「零实现」，须用户裁决 |

预计 pending：477 → 383（移除全批准则 373）。

## 转场景清单（91，按能力；⭐=高价值守卫）
- app-tui-host(8)：r1248 teardown 探针 / r1270 最小尺寸提示 / r1279 bang 中 agent 流+bash 输出按需渲染+Esc 可 abort /
  r1244 GetMessages 失败显错⭐ / r1247 Resume 面板删除当前会话拒绝+删除须确认⭐ / r1253 idle 纯 Tick 帧不变 /
  r1258 Mouse Moved 无态变不重绘 / r1262 空输入 Enter 仅滚底
- app-tui-session-tree(5)：r1326 树内 Shift+F 分叉⭐ / r1327 树槽 Search/TreeHelp 行 / r1328 注解编辑→Label /
  r1329 /debug 树 fixture / r1336 真实回合后树非空
- app-tui-input(6)：r1316 跨会话 ↑ 历史 / r1298 /model 补全 / r1304 Resume 键矩阵⭐ / r1309 @ 路径补全 /
  r1310 粘贴占位提交展开⭐ / r1315 $ skill 补全
- app-tui-transcript(1)：r1347 超大 diff 截断+omitted
- app-tui-trust(1)：r1375 ChoicePrompt Esc=deny
- layer-architecture(2)：r1511 reload 双端同时生效 / r1512 print 无监听器同进程完成
- agent-runtime(3)：r1040 Done usage 落盘⭐ / r1042 两轮间切模型第二轮用新模型⭐ / r1057 mid-run set_tools 下轮生效
- agent-todo(7)：r1119 压缩重追加 todo 快照⭐ / r1123 Custom latest-wins / r1125 两工具语义+未知 id 拒⭐ /
  r1126 多 in_progress / r1121 导出含 todo 标记 / r1842 超 80 拒绝⭐ / r1122 成功→TodoUpdated 事件
- agent-hooks(1)：r1005 model_select/thinking_level_select hook
- agent-prompt(4)：r1023 apply_prompt_resources 下轮生效 / r1024 apply_skills / r1025 $name 注入正文⭐ /
  r1017 Available tools 无 mcp 名+引导句
- package-ai-bridge-accounting(2)：r1561 计量优先级链 / r1564 abort usage 不作锚点⭐
- agent-tools(2)：r1149+r1153 工具 schema timeout 面
- runtime-model-registry(2)：r1744 thinking-level 注入请求体 / r1745 零模型硬错误（均为既有步直配）
- runtime-resource-discovery(7)：r1759 r1760 r1761 r1762（既有步直配）+ r1763 r1764 r1765（resources Info/Doctor/只读，新步）
- infra-bash(5)：r1430 abort cancelled / r1432 bash 记录行 / r1433 bang 路由 / r1434 exclude 过滤 / r1427 bash hook
- infra-mcp(4)：r1445 无配置零装配 / r1447 fixture 装配 / r1448 动态重载 / r1449 无效条目诊断
- infra-provider(2)：r1498 SSE→chunk / r1504 input items 带 type
- infra-observability(2)：r1462 raw/mapped 成对 / r1463 闸 on/off
- infra-otel(6)：r1470 闲置独立根 / r1473 io=full 请求体 / r1474 abort ERROR/aborted⭐ / r1475 并行同 trace /
  r1488 双闸 fan-out / r1492 usage_details 无 gen_ai
- domain-compaction(3)：r1411 force instructions / r1415 压后地板诊断⭐ / r1416 policy 指纹
- protocol-app(1)：r1693 Command 往返
- server-core(1)：r1790 session/resources 下行一帧不占 seq⭐
- cli-entry(2)：r67 get_commands 字段 / r68 source 枚举
- cli-print(5)：r34 流式 / r43 工具摘要 / r49 thinking→stderr / r52 无重复前缀 / r55 非零退出⭐（该能力现零场景）
- user-experience(4)：r1838-r1841 引导消息族（provider_guidance.rs 已实现）
- test-provider-integration(3)：r1821/1822/1823 ConfigValue 三形态+插值+! 命令
- package-tui-interaction-modes(2)：r1628 拖选复制 / r1631 dock 排除（既有步直配）

## 移除候选（10，须裁决）
1. infra-network 整系 6 条（r1455-r1460）：全仓零实现（httpProxy/HTTP_IDLE_TIMEOUT 无符号）；
   r1460 与 runtime-config r1724「settings 仅交付面字段」场景直接冲突。建议整系移除；
   若想留作 roadmap，转 llman-sdd-draft。
2. r1754 作用域模型（runtime-model-registry）：无 --models/scoped_models 实现；模型循环已由
   agent-session cycleForward 场景覆盖。建议移除或转 draft。
3. r1757 BDD-model 全通过（runtime-model-registry）：纯元规则，与 test-qa-gate 门禁重复。
4. r31 快照六操作（agent-session-store）：snapshot/restore/spawn/prune/diff/merge 无任何实现。
5. r1089 分享指引（agent-session-store）：share 功能未实现。

## 执行形态（批准后）
1. `llman-sdd change new` → `change start` 绑定非默认分支（specs 改动禁止直落 main）；
   本计划与决策稿（/tmp/wave2_decisions.md）入 change research/。
2. 按域分批落地（app-tui → agent/bridge → runtime/infra → domain/cli → package-tui），逐批 commit；
   裸规则转场景只增 `场景:` 嵌套，不改规则正文；合并写明 absorbed-by；移除仅动候选清单批准项。
3. 门禁：每批 `validate --specs --strict` 全绿 + review pending 单调下降对账；收口 `just qa` 全绿；
   staleness 按 scope 提交对应 .feature 编辑（沿用第一波 re-review 戳纪律）。
4. `change finalize` 归档，附 pending 前后对比与四类决策计数。
