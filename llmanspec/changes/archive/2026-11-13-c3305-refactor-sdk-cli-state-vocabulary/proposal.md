---
depends_on: []
title: "Consolidate SDK and CLI state vocabulary"
needs_specs_change: false
branch: sdd/c3305-refactor-sdk-cli-state-vocabulary
base_branch: main
base_sha: 6ca95255396775ca40f3ff0c5aec2f9720dc4317
---

# SDK 与 CLI 状态词表收敛（P2 草案）

## Why

2026-11-10 隐式状态机扫描的低风险 SDK/CLI 项集中收敛（详细点位见扫描报告；本 change 为草案，propose 细化时逐条复核）：

- XyEvent 配对仅靠发射顺序，CompactionEnd 双 bool + 可选字段可构造矛盾组合（F6）。
- surface 意图 4 bool + 两套判据已漂移（F7）；trust 结果 override/store/warning 三处表达、--trust/--no-trust 冲突不报（F8）。
- obs_gate thread_local 遮蔽全局原子、作用域内 set_* 静默丢弃（F9）；settlement 全局自增号无「当前」语义（F10）。
- loop 七阶段在 SDK 3 处并列，声明某 stage 未覆写方法时静默 no-op（F14）。
- capability 4 bool 中 2 个恒为派生值且 harness 直接改写（F15）；压缩快照 Option 群 + 魔法字符串（F16）；压缩归属双 bool 中 thinking_rejected 无通知分支（F17）。
- obs 插件 OnceLock 诊断 + 三个全局位，运行期重配不可复位（C19）。
- 图边「最新记录即当前态」删除语义（F21）、树旅行动作字段组合（F22）、状态栏 None/空列表双表示（F23）、Option<bool> 三态只用两义（F24）、env/prepare 追加判据各看不同字段（F25）、codemode 双闸只告警（F20）。

## What Changes（方向）

- 事件配对与结果结构显式化（矛盾组合不可构造）；意图解析单谓词；obs 作用域与全局位单一真值；七阶段声明-实现校验；capability 派生字段去冗余。

## Impact

- crates/xylitol-plugin-sdk、src/cli*、plugins/xylitol-official-plugin-obs。纯代码组织与词表收敛，无产品行为变更。

## Further Notes

- 落地前以扫描报告逐条复核；F8 的冲突报错若算行为变更，拆出到独立 fix change。
