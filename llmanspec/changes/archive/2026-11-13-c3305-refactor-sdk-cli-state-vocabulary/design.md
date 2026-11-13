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
| F6 | CompactionEnd 枚举化 `Completed / Failed{error, will_retry} / Aborted`（sdk lifecycle + wire 映射 + compaction 插件 + TUI handler 同步）；TUI 死分支删除 |
| F7 | SurfaceIntent 显式结构；select_surface_mode 与 cli.rs 判据合一 |
| F8（本单部分） | project_trusted 去 warnings 反推；冲突报错拆独立 fix change |
| F9 | obs_gate set_* 命中 scope 时 debug_assert/warn 或拆显式 scoped_set；先以单测钉死现语义（scope 内写不落全局、随 scope 丢弃）；触发面现仅测试 |
| F10 | 倾向 per-session 槽内单调号 + driver 暴露 current_generation，消费端可判陈旧；或有人消费前删事件字段 |
| F14 | 枚举 strum 化（EnumIter/EnumString）替代 ALL+as_str；SDK 提供作者侧七阶段全声明探针测试（runtime 验证各桶命中）；默认实现保留 |
| F15 | 序列化形状不动；唯一构造 `CapabilityEntry::new(id,kind,label,session_disabled,applied)` 内部派生三 bool；harness 复用同构造；SDK 恒等式单测 |
| F16 | SDK 补 `CUSTOM_TYPE_CONTEXT_FINGERPRINT` 常量（双插件共用）；reason 改 CompactionReason 枚举（wire 文本不变）；快照校验构造；根 AGENTS 插件清单补 context-inject |
| F17 | thinking_rejected 补通知分支（与 fallback 同管线） |
| F20 | is_active≡enabled 收敛；冲突至少升格 BootstrapWarning（现仅日志可见）；硬报错/自动启用属产品行为变更，超出本单 |
| F21 | 图边删除语义显式（墓碑 enum 或 enabled 语义注释收紧；生产写点仅 fork 恒 true） |
| F22 | `TravelAction{Rewind{prefill}, View}` 枚举（wire 文本不变）；端侧 session_ops 停止组合推断 |
| F23 | 状态栏类型收窄为单一表示（去防御双臂） |
| F24 | from_hook 双字段全仓只写 None/Some(false) 且零读者——删除或 serde default 兼容收窄；保留三态须先有消费语义 |
| F25 | env/prepare 追加判据合一（date+cwd+fingerprint 单谓词） |

## 3. 实施顺序

编译期可保证的先行（F6/F14/F15），运行时收敛居中（F9/F10），rider 收尾；F16 常量单声明与 AGENTS 清单补录同任务。见 tasks.md。

## 4. 测试边界

单测为主（构造约束、谓词、真值表）；F17 通知分支补插件单测；BDD 仅在发现行为回归时补。
