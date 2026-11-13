# Design — SDK 与 CLI 状态词表收敛

## 0. 复核结论（2026-11-13，HEAD 7d36d7e 逐条开文件核对）

16 点位：成立 14、表述需修正 2（F14 计数、F23 性质）、不成立 0、已被修复 0。c3275–c3300 与本批同区域（c3290 触 F16/F17 文件、c3300 触 F20 插件）但均不在其范围，全部点位现码仍成立。

修正两处：

- F14 七阶段并列声明实为 **4 处**（loop_middleware.rs:14-22/25-33/35-45/378-429）；未覆写静默 no-op 双向成立（默认实现 :191-217 + snapshot 仅收声明桶 :339-341）。
- F23 状态栏双表示：类型允许（status_bar.rs:9）但生产已归一（:15）且 render 双臂收拢（:20-24）——实态 = 防御成本，非行为歧义；收敛为类型层收窄即可。

关键成立点：F6 CompactionEnd 四 emit 全硬编码 aborted:false（orchestrator.rs:58/104/203/425），TUI aborted 分支为死分支；F7 surface 4 bool（intent.rs:113-129）与 cli.rs:96-100 两套判据；F8 --trust/--no-trust (true,true) 静默取 true（cli.rs:88-92）、args 无 conflicts_with、override/store/warning 三表达（resolve.rs:36-47/57-62、bootstrap.rs:478-488）；F9 obs_gate thread_local 遮蔽全局原子（obs_gate.rs:74-117），生产写点仅 obs init_logging、scope 进入全在 cfg(test)；F10 settlement 进程级自增号全仓零比较（消费端 lifecycle.rs:137、print.rs:111 全忽略）；F15 capability 4 bool 中 **3 个非独立**（构造恒等 capability.rs:54-58/71-74；harness 直改 = app-tui 测试 harness.rs:760-779）；F16 "context-fingerprint-snapshot" 常量双声明（context-inject drift.rs:15 vs compaction lib.rs:453）——根因 = context-inject 已是 workspace 成员（Cargo.toml:35）但根 AGENTS 官方插件清单未列；F24 from_hook 全仓只写 None/Some(false) 且零读者。

## 1. 目标与边界

- SDK/CLI 词表与状态载体收敛；无产品行为变更（needs_specs_change: false）。
- 边界：XyEvent 变体形状不动（只收紧可构造组合）；配置格式不动。
- **F8 拆出**：--trust/--no-trust 冲突显式报错属行为变更，不在本单；本单只做 project_trusted 不再由 warnings 反推。拆出项记 proposal Further Notes。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| F6 | CompactionEnd 收紧：aborted/will_retry 与 summary 矛盾组合不可构造（构造器断言或类型分叉）；TUI 死分支删除 |
| F7 | SurfaceIntent 显式结构；select_surface_mode 与 cli.rs 判据合一 |
| F8（本单部分） | project_trusted 去 warnings 反推；冲突报错拆独立 fix change |
| F9 | obs_gate span_sink 与 ACTIVE 计数单一结构；作用域 set_* 改记录待生效（触发面现仅测试） |
| F10 | sdk 提供 is_current 谓词；消费端显式忽略需注释豁免或改用谓词 |
| F14 | 4 处并列声明收敛为一；声明-覆写注册期校验（未覆写不再静默 no-op） |
| F15 | 3 个派生 bool 去字段，快照函数统一产出；测试 harness 走正式构造 |
| F16 | Option 群收敛 + 魔法串常量单声明（context-inject / compaction 共用 sdk 常量）；根 AGENTS 插件清单补 context-inject（文档直改） |
| F17 | thinking_rejected 补通知分支（与 fallback 同管线） |
| F20 | codemode 双闸 is_active≡enabled 收敛（codemode.rs:126-129），kernel 侧 warn 保留 |
| F21 | 图边删除语义显式（墓碑 enum 或 enabled 语义注释收紧；生产写点仅 fork 恒 true） |
| F22 | 树旅行动作改显式枚举，端侧停止组合推断 |
| F23 | 状态栏类型收窄为单一表示（去防御双臂） |
| F24 | from_hook 死三态收敛或删除零读者分支 |
| F25 | env/prepare 追加判据合一（date+cwd+fingerprint 单谓词） |

## 3. 实施顺序

编译期可保证的先行（F6/F14/F15），运行时收敛居中（F9/F10），rider 收尾；F16 常量单声明与 AGENTS 清单补录同任务。见 tasks.md。

## 4. 测试边界

单测为主（构造约束、谓词、真值表）；F17 通知分支补插件单测；BDD 仅在发现行为回归时补。
