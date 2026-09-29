# A 组（app-tui 域，93 条）
## layer-architecture：转2 保25
- 转: r1511(组合场景:/reload 客户端键位热载+host 运行时重载恰一次,双端同时生效), r1512(print 无监听器同进程正常完成一轮)
- 保(d 结构/反 grep 元测试,规范自身禁止): r1520 r1525 r1531 r1533 r1535 r1521 r1522 r1523 r1524 r1526 r1527 r1519 r1518 r1516 r1528 r1529 r1530 r1507 r1508 r1509 r12 r1510 r1513
- 保(a): r1534(Agent::new 签名单测); 保(c): r1517(需外部 crate 编译验证)
## app-tui-host：转8 保18
- 转: r1248(/exit 后 teardown 探针 raw/alt/mouse 复原,执行时核探针), r1270(小于最小尺寸终端显示友好提示), r1279(bang 中 agent 流+bash 输出按需渲染且 Esc 可 abort), r1244(注入 GetMessages 失败→错误 note 非静默空 transcript,需 driver 失败旋钮), r1247(Resume 面板删除当前会话被拒+删除须确认), r1253(idle 无 dirty 纯 Tick 帧逐字不变), r1258(Mouse Moved 无态变不重绘帧对比), r1262(Ready 空输入 Enter 仅滚底不提交)
- 保(b): r1245(resume 列表经缝,atm 场景族), r1265(attach mux 续传,remote-resilience+resync 场景), r1266(恢复投影,resume 投影场景), r1267(reload 合作取消,ath 已取消场景), r1268(下行驱动刷新,resources 快照场景), r1273(重连宽限 UX,remote-resilience 宽限场景)
- 保(c): r1254(需条目重绘/miss 计数钩子), r1255(需流式全量解析计数钩子), r1264(需可控慢 Host 时序), r1269(需慢 Host 有界等待 harness)
- 保(d): r1240 r1277 r1278 r1241 r1243 r1282 r1260; 保(a): r1259
## app-tui-session-tree：转5 保9
- 转: r1326(树内 Shift+F→child 会话含父头并切换,组合既有 fork 步), r1327(树槽 Search/TreeHelp 行渲染), r1328(树内 Shift+L 注解→Label 写入且重开可见), r1329(/debug session-tree-* 后树非空含 fixture), r1336(真实跑完一轮后双 Esc 树含该回合节点)
- 保(b): r1325(slot 替换,app-tui-input 场景), r1332(活树 travel,app-tui-input), r1337(过滤模式矩阵,app-tui-input), r1338(折叠转发,树槽和弦场景), r1330(尾插通告,app-tui-transcript)
- 保(a): r1333 r1334 r1331(agent_demo 包单测), r1335(映射纯函数)
## app-tui-input：转6 保4
- 转: r1316(跨会话 user 正文进 ↑ 历史,新步), r1298(/model 空格弹补全 Tab 写入 Esc 不调 SetModel), r1304(Resume 面板键矩阵:scope/排序/rename/删除确认+busy Enter 拒绝,2-3 场景), r1309(@ 路径补全选定写入 Esc 不改文本), r1310(粘贴占位提交时 Driver 收展开全文), r1315($ skill 补全+scrollback 高亮)
- 保(b): r1314(已信任无逐工具审批,回合场景族); 保(a): r1322 r1323 r1324(demo 包单测)
## app-tui-transcript：转1 保3
- 转: r1347(超大 display_diff 截断+omitted 提示)
- 保(d): r1360 r1369 r1370(复用mandate/反模式;行为已被 diff/rail 场景覆盖)
## app-tui-trust：转1 保3
- 转: r1375(ChoicePrompt Esc=deny 不写盘,确认才写 store)
- 保(b): r1374(ChoicePrompt trust 场景族), r1376(同 r1314), r1377(/trust 缝持久化不自动重载,既有步)
## app-tui-commands：保2  (d): r1199 r1203
## app-tui-bridge：保1  (b): r1187(桥缝 bash 增量块场景已有)
## app-tui：保5  (d): r1164 r1165 r1166 r1163 r1162
# B 组（agent/bridge 域，97 条）
## package-ai-bridge：转0 保20
- 保(a 规范自declared包内单测): r1555 r1556 r1545 r1547 r1549 r1550 r1551 r1552 r1553 r1554 r1902 r1558
- 保(d): r1536 r1546 r1557 r1560 r1543 r1548 r1537; 保(b): r1559(project_for_llm 投影,agent-session 场景)
## package-ai-bridge-accounting：转2 保7
- 转: r1561(计量优先级链一场景:锚点在→Api,否则降级链), r1564(abort/error 回合 usage 不作 Api 锚点)
- 保(b): r1566(Heuristic 标注既有步) r1567(降级通知既有步) r1568(tokenizer 缓存场景族) r1562(local_tokenizer 闸既有步); 保(a): r1563 r1565 r1569
## agent-runtime：转3 保10
- 转: r1057(回合进行中 set_tools 不影响本轮下轮生效), r1040(Done usage/stop_reason 落持久 assistant), r1042(两轮间切模型第二轮请求用新模型)
- 保(b): r1049(threshold 场景) r1050(overflow 场景,规范自指 domain-compaction 锚点) r1053(kind 既有步); 保(a): r1055 r1031 r1032 r1051 r1052 r1054; 保(d): r1056
## agent-todo：转7 保5
- 转: r1123(todo 写入→Custom 全量快照 latest-wins), r1125(两工具语义+未知 id 拒+仅两 todo 工具), r1126(两条 in_progress 写入成功), r1119(压缩裁掉 todo 快照后重追加,高价值), r1121(导出含 agent_todo 标记非用户消息), r1842(超 80 拒绝不截断), r1122(todo_update 成功→TodoUpdated 事件)
- 保(a): r1118 r1124 r1127 r1120; 保(d): r1128
## agent-session-store：转0 保7（含 2 移除候选）
- 移除候选(需核实现): r31(快照 snapshot/restore/merge 疑未实现), r1089(分享指引疑未实现)
- 保(b): r41(threshold 场景) r1114(resume cwd 校验场景) r1117(journal 场景) r1116(导出场景); 保(d): r1115
## agent-session：转0 保4
- 保(d): r1077 r1080; 保(b): r1083(as_agent_message/context 场景); 保(a): r1084(规范自declared)
## agent-tools：转2 保10
- 转: r1149(grep/find schema 含可选 timeout), r1153(fs 四工具 schema 无 per-call timeout+默认 30s 呈现)
- 保(b): r1151(mcp barrier 场景) r1152(ask 场景族); 保(a): r1150; 保(d): r32 r10 r1134 r1141 r1142 r1143 r36
## agent-hooks：转1 保9
- 转: r1005(SetModel/CycleModel→model_select hook 上下文,既有「上下文含键 model」步)
- 保(b): r1000 r1008 r1011 r1001 r1002 r1003 r1004 r1006 r1007(各事件 hook 场景族全有步)
## agent-prompt：转4 保6
- 转: r1023(apply_prompt_resources 下轮生效,与 r1057 同族), r1024(apply_skills 重建 available_skills), r1025($name 注入 SKILL.md 正文历史留原文), r1017(Available tools 无 mcp 名+含引导句)
- 保(b): r1015 r1020; 保(a): r1022 r1016 r1018 r1019
# D 组（domain/cli/test 域，79 条）
## infra-otel：转6 保6
- 转: r1470(闲置路径 token.estimate 独立根带 session id), r1473(io=full 时 llm.request input=请求体 JSON 截断), r1474(TextDelta 后 abort→llm.request ERROR/aborted+flush), r1475(并行窗两 tool.execute 同 trace 同 iteration), r1488(双闸开 file+collect 同收一批), r1492(带 usage Done→usage_details 且无 gen_ai 键)
- 保(a 规范declared): r1481 r1482 r1483 r1484 r1485; 保(d): r1487
## domain-compaction：转3 保6
- 转: r1411(force compact 带 instructions→Additional focus 且骨架保留), r1415(压后仍无窗口→地板诊断每 run 一次), r1416(CompactionEntry policy 快照四字段+legacy 不当当前配置)
- 保(b): r1402(同源估计触发场景族) r1410(overflow/threshold 分流场景) r1412(仅 leaf 分支既有步) r1413(settlement 恰一次既有步); 保(a): r1414; 保(d): r1400
## domain-security：保3
- 保(b): r66(网络域既有步); 保(d): r71 r1425
## protocol-app：转1 保4
- 转: r1693(Command 变体线协议往返保真)
- 保(b): r1695(r1698 Event 映射场景) r1703(bash 独立路径既有场景) r1705(反 REST 场景族); 保(d): r1694
## server-core：转1 保1
- 转: r1790(资源快照变化→session/resources notification 一帧不占 seq)
- 保(b): r1779(既有 estimate_context unary 场景,执行时核对固定开销断言)
## cli-entry：转2 保4
- 转: r67(get_commands 条目含 name/description/source/source_info), r68(source 恰为 prompt|skill)
- 保(b): r1383(run_with_id 场景族复用 session id); 保(d): r1379 r1392 r1394
## cli-print：转5 保0
- 转: r34(流式 stdout) r43(工具名+结果摘要行) r49(thinking 进 stderr 包 think 不双包) r52(增量无前缀重复) r55(Error 非零退出+工具单败不退出)——该能力现零场景,经 CliEntryBdd+fake provider 全可驱动
## user-experience：转3（含 2 需核实现）+1 移除候选
- 转: r1840(UNSET_MODEL_DISPLAY 未选模型提示既有面); r1839(无可用模型提示,核实现) r1841(无 api key 提示含 provider 名,核实现)
- 移除候选: r1838(login-help 疑未实现,需核)
## test-qa-gate：保7  (d 流程/资产契约): r1827 r1828 r1829 r1830 r1831 r1832 r1833
## test-infra：保6  (b): r39(FauxProvider 场景族); (d): r46 r54 r57 r60 r63
## test-provider-integration：转3 保3
- 转: r1821(r183 三形态共享场景族) r1822($VAR/${VAR:-default} 插值) r1823(! 命令解析,超时/缓存细节归单测)
- 保(b): r1824(runtime-model-registry/config 场景族); 保(d): r1825 r1826
## test-bdd：保5  (b): r1811(server/approval 步骤已在); (d): r37 r1812 r1813 r1814
## test-standards：保4  (d): r1834 r1835 r1836 r1837
## test-hooks-wiring：保3  (b): r1817(agent-hooks 场景族); (d): r1818 r1819
## test-fake-provider：保2  (b): r38 r45(mock 场景族 delay/错误注入全有)
# 补遗：app-tui-fixed-zone（4 条）
- 保(b): r1233(glyph 显式切换既有步,运行时探测禁令为负向) r1231(footer 无队列徽章既有断言)
- 保(a): r1235(demo 主题探测,包单测); 保(d): r1234(文档 SSOT)
# 修正：user-experience 四条全部有实现（provider_guidance.rs 纯函数+单测）
- 转: r1838(login 引导消息存在) r1839(无可用模型→引导) r1840(未选模型提示) r1841(无 api key 含 provider 名)
# 最终汇总
- 转 91 / 并 3 / 保 373 / 移除候选 10 = 477
- 移除候选(10): infra-network 整系 r1455-r1460(6,零实现且 r1460 与 r1724 冲突), r1754(作用域模型,零实现), r1757(BDD-model 元规则,与 qa 门重复), r31(快照六操作,零实现), r1089(分享指引,零实现)
