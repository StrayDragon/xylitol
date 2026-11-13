---
depends_on: []
title: "Fix print thinking block tag pairing"
---

# Tasks — c3320 Print 思考块标签配对

T1→T2→T3→T4 串行（T3 断言依赖 T1 注入点，T2 修复依赖 T1 才可断言）。

- [ ] T1 render_stream 新增可注入 err sink，run_print 默认接 io::stderr()；行为不变。验收：`cargo nextest run -p xylitol-app-print` 现有测试全绿 + 新增双缓冲单测（err 恰一对标签）。
- [ ] T2 ThinkBlockPhase 枚举化：单点开闭、MessageStart 先收口、per-fragment 闭合检测、首片开标签判定保留。blocked_by: [T1]
- [ ] T3 BDD r49 扩场景（多消息打断、标签跨片）+ 修 steps_layer_invariants.rs:279-280 fixture 同串 bug。blocked_by: [T2]
- [ ] T4 收口：`llman-sdd validate c3320 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出；改动行不得新增可移除注释）。blocked_by: [T3]
