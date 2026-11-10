# Design — SDK 与 CLI 状态词表收敛

## 1. 目标与边界

- 扫描低风险项的 SDK/CLI 词表与状态载体收敛；无产品行为变更（needs_specs_change: false）。
- 边界：XyEvent 变体形状不动（只收紧可构造组合）；配置格式不动。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| XyEvent 配对 | CompactionEnd 构造器收紧（aborted/will_retry/summary 矛盾组合不可构造，F6） |
| surface 意图 | select_surface_mode 输入改显式 SurfaceIntent 结构；tui_requires_host_listener 与 cli.rs 判据合一（F7） |
| trust 冗余 | --trust/--no-trust 冲突显式报错；project_trusted 不再由 warnings 反推（F8，若算行为变更拆出） |
| obs_gate | 作用域内 set_* 不丢弃改为记录待生效；span_sink 与 ACTIVE 计数单一结构（F9） |
| settlement | generation 比较逻辑随 sdk 提供谓词 is_current（F10） |
| 七阶段 | LoopStage 声明与方法覆写编译期/注册期校验（trait 默认方法 + 注册时断言，F14） |
| capability | effective/profile 派生字段去冗余，快照函数统一产出（F15） |
| 其余 rider | F16/F17/F20-F25 按扫描点位逐条收敛（Option 群、图边、树动作、状态栏、追加判据） |

## 3. 实施顺序

先编译期可保证的（配对收紧、七阶段校验），再运行时收敛（obs/settlement），最后 rider 清单。逐条独立小 PR 粒度，可按需拆细。

## 4. 测试边界

单测为主（构造约束、谓词）；BDD 仅在 F8 拆出行为变更时补。
