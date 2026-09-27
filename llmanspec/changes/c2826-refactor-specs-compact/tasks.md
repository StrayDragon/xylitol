# Tasks

- [ ] T1 换号修复：package-ai-bridge 内重复 `@req:r1555`，deepseek-prompt-cache-read-mapped 换 r1902
- [ ] T2 agent-session-store：12 转场景 + 合并 r1113→r1112（bindings/steps 落地，测试绿）
- [ ] T3 protocol-app：13 转场景 + 承接 r1034 合并补句（bindings/steps 落地，测试绿）
- [ ] T4 cli-entry：15 转场景 + 合并 r1380→r70 + 承接 layer r1515 合并（bindings/steps 落地，测试绿）
- [ ] T5 infra-otel：14 转场景（bindings/steps 落地，测试绿）
- [ ] T6 agent-runtime：10 转场景 + 合并 r1033→store r1094、r1034→protocol-app r1698（bindings/steps 落地，测试绿）
- [ ] T7 package-ai-bridge：3 转场景（bindings/steps 落地，测试绿）
- [ ] T8 app-tui-commands：14 转场景 + 合并 r1200→r1188（bindings/steps 落地，测试绿）
- [ ] T9 app-tui-fixed-zone：22 转场景 + 合并 r1218→r1224、r1225→ath r1251（bindings/steps 落地，测试绿）
- [ ] T10 app-tui-input：21 转场景 + 合并 r1320→r1286（bindings/steps 落地，测试绿）
- [ ] T11 app-tui-host：29 转场景 + 合并 r1242→r1280；layer-architecture：2 转场景 + r1515 移除（bindings/steps 落地，测试绿）
- [ ] T12 全量门禁：`llman-sdd validate --specs --strict` 全绿；review pending 与计划对账；`just qa`；finalize 收口
