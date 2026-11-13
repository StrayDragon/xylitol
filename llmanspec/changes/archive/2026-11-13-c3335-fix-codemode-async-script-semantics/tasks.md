---
depends_on: []
title: "Fix codemode async script semantics"
---

# Tasks — c3335 run_code 异步函数体与结果信封

- [x] T1 spec 落地：agent-codemode.feature 增 `script-body-and-result` 规则 + 三场景（change 分支上提交）。
- [x] T2 guest 异步：脚本包装改 async IIFE + 微任务泵至 Promise settle；顶层 await（含 await tools.<name>）单测。验收：`cargo nextest run -p xylitol-official-plugin-codemode` 绿。
- [x] T3 完成信封：completed/failed + 耗时 + 有序输出项；undefined 显式无输出标识；真 null 渲染 `return: null` 可区分；截断保信封首行；裸 "null" 禁止。blocked_by: [T2]
- [x] T4 text() 输出通道：preamble 升格发射 + 序列化不变；console 块保留。blocked_by: [T2]
- [x] T5 描述与提示词：tool.rs / declarations.rs 写明 body/await/return 与输出通道。blocked_by: [T3, T4]
- [x] T6 BDD 与回归：新场景绑定；r3253 嵌套 dispatch、r3254 限额错误分类回归。blocked_by: [T1, T3, T4]
- [x] T7 收口：`llman-sdd validate c3335 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）。blocked_by: [T5, T6]
