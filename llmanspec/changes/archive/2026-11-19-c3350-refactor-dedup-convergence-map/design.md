# Design

## 地图结构（决策架构）

目的地：四个重复收敛点各有完结决策——每项定落点、公共形状、合约影响、实施路径（propose / quick）——决策无遗留，分别转 propose / quick 实施。

```text
已关闭（决策输入）
  c3355 sdk 收纳边界（四条准则 + sdk→protocol 边，src/AGENTS.md 已落）
  c3360 provider 依赖面逐件判定（真实重复 839 行/份；obs_session 为 facade 非重复）
  c3365 截断合约钉死面 / 可统一面（r1431 三层逐字钉死；r1140 漂移交 c3380）
  c3370 dispatch 推荐 plugin-sdk（sdk→protocol 边 + 5 数据类型下沉 + kind 化错误构造）
待决策（本图输出）
  c3375 provider 脚手架归属 ←─ c3355 + c3360
  c3380 截断策略模块形状   ←─ c3355 + c3365
  c3385 dispatch 合约归属   ←─ c3355 + c3370
  c3390 lsp/dap 生命周期组件 ←─ c3355
```

## 各票决策输出（待产出）

- **c3375**：逐件归属清单（trace / wire_policy / thinking / json_parse / attribution / reqwest_headers / 壳），is_opencode_host 处置，thinking 收敛深度，新方言插件脚手架成本基线，实施路径。
- **c3380**：截断策略模块接口形状（模式 / 上限 / 通知模板 / 元数据）、50KiB 常数单点归属（r2934「不得复用观测 IO 常数」并存）、通知文案路线（组合式锚行 vs propose 改 r1431/r1345）、r1140 漂移处置、迁移顺序。
- **c3385**：翻译合约单点落点确认（拟 sdk：DispatchOutcome / SessionCommandExecutor / dispatch() / validate / transport_variant_error / variant_name）、5 数据类型下沉清单、错误 kind 化最小面、新增 Command 改动点数预期、kernel outcome_to_value 去留。
- **c3390**：NotStarted/Running/Dead 相位机 + 可注入传输错误分类器接口、lsp/dap backend 注入形态、分类器脱离真实 server 的单测形态、c3300 表述承接或重开。

## 关闭与收拢协议

- 每票：逐问深挖 → 决策写该票 proposal（Open Questions）→ change start/archive 关闭 → 本图 Decisions so far 追加一行。
- 全部归属票关闭后：解决「实施分组」（一张收拢 propose vs 每决策各走 propose/quick），转 llman-sdd-propose 收拢为可实施计划；本图随之 finalize/archive。
- 术语校准与 spec 触碰：归属票决策若需要改 specs，须在该票绑定分支提交；本图自身不改 specs（needs_specs_change: false）。
