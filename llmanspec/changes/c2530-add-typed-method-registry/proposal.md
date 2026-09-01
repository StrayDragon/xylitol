---
depends_on: []
skip_specs_landing: true
branch: sdd/c2530-add-typed-method-registry
base_sha: 2c04ff709430795d77101d1e632ae66080ee80db
checkpointed: false
---

# Typed Method Registry：unary 方法单点声明注册表

## Why

当前新增/修改一个 unary 方法要人工同步 **5 处**，且类型检查缺位：

1. `UNARY_METHODS` 裸字符串表（`src/protocol/wire/method.rs`）；
2. `handle_unary` 特例分支（`src/app/server/host.rs`）——顺序敏感，「abort 旁路幂等准入」靠**代码行位**保证（`host.rs:1199` 在 `admit()` 之前 return）；
3. method→Command 手搓转换表（`host.rs:899` 起 ~40 arm），逐字段从 `Value` 抠参数，字段名打错**静默落默认值**（如 `last_seq unwrap_or(0)`）；
4. `dispatch.rs` 的 Command match（已类型化，但与方法名无编译期绑定）；
5. 客户端 `driver/remote.rs` 手搓 `json!({...})` 载荷，字段名打错即运行时 bug。

后果：导航靠 grep 字符串（rust-analyzer 不可跳转）；载荷演进无编译期护栏；OpenAPI 只有方法名没有 schema；未来第二客户端（gpui / Rust-wasm / TS）每家都要重走一遍字符串接缝。

选型结论（见 `research/protocol-selection-notes.md`）：协议与载体不变，收益全部在「一处声明、多处派生」的组织形态。

## What Changes

- 新增声明式方法注册表（宏驱动）：单点声明 `方法名 ↔ Command 变体 ↔ handler ↔ 请求/响应类型 ↔ 能力位`；
- 从注册表派生：方法表与 404 语义、请求解析与 serde 校验（替换手搓转换表）、salvo 路由绑定、进程内 port 入口、客户端类型化调用方法；
- 响应种类成为声明能力位：`result`（同步）/ `stream`（ack + 事件走 mux）/ `job`（ticket，预留）；bash/reload 后续毕业为作业语义（本 change 只预留位，不实现 job 语义）；
- 顺序敏感语义显式化：「abort 旁路幂等」从代码行位改为 handler 能力声明；
- `Command` 枚举保留为 port 级词表（进程内 XyDriver 消费路径不变），注册表保证方法名 ↔ Command ↔ handler 一一对应，不产生第二 SSOT。

## 非目标

- 不换协议/载体/框架（POST + WS + JSON 宪法不变）；
- 不改任何 wire 行为：信封形状、幂等语义、事件闭集、seq/重放合约全部不动，**既有 BDD 全绿 + 既有单测全绿为验收**；
- 不引入跨语言 codegen 子系统（TS descriptor 发射只留数据钩子，另行立项）。

## Capabilities

- 无 live 合约变更（`skip_specs_landing: true`）：本 change 为行为保持的组织形态重构，specs 约束层级规则下不落 `.feature`；相关既有 capability：`protocol-app`、`server-core`、`layer-architecture`（均不修改）。

## Impact

- 主战场：`src/app/server/host.rs`（手搓表与特例迁移）、`src/protocol/wire/method.rs`（表派生化）、`src/app/core/driver/remote.rs`（客户端类型化调用）、新增注册表宏模块；
- 迁移策略 expand-contract：注册表与旧路径并存 → 分批迁移方法 → 删除手搓表；每批以既有 BDD + 单测对拍（seam 全部复用既有 harness，不新增 seam、不新增 `.feature`）；
- OpenAPI 文档从「仅方法名」升级为带 schema（调试面改善，非破坏）。

## Further Notes

- 选型分析全文与宪法：[research/protocol-selection-notes.md](./research/protocol-selection-notes.md)
- tonic 对照探针报告：[research/lab-tonic-probe-report.md](./research/lab-tonic-probe-report.md)（待补）
