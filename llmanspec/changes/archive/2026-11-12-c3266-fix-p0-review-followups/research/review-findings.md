# P0 批次三组 review 证据摘要（2026-11-12）

三组只读 reviewer（runtime 批 / 存储-服务-provider 批 / 门禁-注入-workflow 批）+ Lead 本机复核。file:line 以当日 main 为准。

## 分级

- CRITICAL：c3265 workflow_write 绕过转移表（tools.rs:149 直收 status；workflow.rs:442-452 只比结构）。
- WARNING：c3240 RunCoordinator transition/mark_streaming 未落（state.rs:150-154 竞态在）；c3260 存量容忍投影未实现（sdk automation.rs:476-479）；c3260 TodoStatus 未知静默 pending（todo middleware.rs:92-98，违反 agent-todo.feature:139）；spec 旧键名事故 ×2（已直改 ec43299）；c3233 manifest 逐条 fsync（append.rs:325,534-550）；c3255 Completions 缺回放测试；c3248 HistoryRow 顺序在用户消息后（react/mod.rs:1010-1011 先落用户 entry）+ session_shutdown 未接线 + session_start=step_index==0；c3248 run_command_capture 超时不杀进程（host_env.rs:26-75）。
- INFO：trust 落点 app-tui-trust；BeforeStepCtx 实 4 字段（session_id 新增，落地 r3313 一致）；drift 无 stat 快路径；压缩保留测试窄；BDD 未端到端；T8 示例未找到；c3265 九字段未降级；checkpoint 静默回扫（checkpoint.rs:84-94）；r2941 spec 漂移（已直改 ec43299，含过时 v7 措辞）；golden 无生成脚本；自适应阈值只算不接；崩溃矩阵近似；v1 allowlist 残留；StreamOutcome 内部形态；flaky 文件被 09df89f 触碰；decide_active_goal 绕表；agent-automation.feature 裸文件布局（先在）。
- 证实为误报：fake Cyclic B21（test_cyclic_mode 反证，不改码正确）。

## 勘误（2026-11-12 本机复核）

- BeforeStepCtx 实为 4 个字段：skills / tools_fingerprint / prepare_fingerprint / session_id（session_id 为 c3240 交付），与落地 spec r3313 一致；上游「3 个资源字段」的旧记不完整，按 4 字段引用。
- c3235 trust 条款落点为 app-tui-trust（端侧 trust 门禁），不在 domain-security 单点；复核检索按该 capability 定位。

## 报告六项待确认裁决

#1 接受（INFO，勘误入本 research）；#2 部分成立（顺序 W7 + W8/W9 两缺陷）；#3 成立；#4 残留回收；#5 breaking 一致，spec 已修，短路列 T9；#6 措辞修正。
