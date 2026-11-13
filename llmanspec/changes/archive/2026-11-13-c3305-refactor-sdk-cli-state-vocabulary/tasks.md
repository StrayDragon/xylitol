---
depends_on: []
title: "Consolidate SDK and CLI state vocabulary"
needs_specs_change: false
---

# Tasks — c3305 SDK 与 CLI 状态词表收敛

每条完成即勾选并跑验收命令；全绿后 validate --strict + just qa。

- [x] T1 F6 CompactionEnd 可构造组合收紧 + TUI 死分支删除。验收：`cargo nextest run -p xylitol-plugin-sdk -p xylitol-app-tui` 绿；矛盾组合构造测试红转绿。
- [x] T2 F14 七阶段 4 处声明收敛为一 + 注册期覆写校验。验收：`cargo nextest run --workspace` loop middleware 测试绿。
- [x] T3 F15 capability 派生 bool 去字段 + harness 走正式构造。验收：`cargo nextest run -p xylitol-plugin-sdk -p xylitol-app-tui` 绿。
- [x] T4 F7 SurfaceIntent 显式结构，判据合一。blocked_by: []
- [x] T5 F8（本单部分）project_trusted 去 warnings 反推；冲突报错行为拆出（记 Further Notes）。blocked_by: []
- [x] T6 F9 obs_gate 单一结构 + 作用域 set_* 待生效语义。blocked_by: []
- [x] T7 F10 settlement is_current 谓词 + 消费端显式化。blocked_by: []
- [x] T8 F16 快照 Option 群收敛 + "context-fingerprint-snapshot" 常量单声明 + 根 AGENTS 插件清单补 context-inject。blocked_by: []
- [x] T9 F17 thinking_rejected 通知分支。blocked_by: [T8]
- [x] T10 F20 codemode is_active≡enabled 收敛。blocked_by: []
- [x] T11 F21 图边删除语义显式。blocked_by: []
- [x] T12 F22 树旅行动作枚举化，端侧去组合推断。blocked_by: []
- [x] T13 F23 状态栏类型收窄单表示。blocked_by: []
- [x] T14 F24 from_hook 死三态收敛。blocked_by: []
- [x] T15 F25 env/prepare 追加判据合一。blocked_by: []
- [x] T16 收口：`llman-sdd validate c3305 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）。
