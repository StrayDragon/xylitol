---
depends_on: []
title: "Consolidate TUI engine component state"
needs_specs_change: false
---

# TUI 引擎组件状态收敛（P2 草案）

## Why

2026-11-10 扫描的 xylitol-tui 引擎中低风险项集中收敛（E5,E6,E8,E9,E10,E11,E12,E13,E14,E15,E16,E17,E18,E20,E21,E22,E23,E25）：

- follow_bottom 缓存漂移（E5）；terminal 6 bool 含只写不读的 kitty_pushed（E6）；editor jump_mode Option<bool> 不可取消（E8）；自动补全双 Option 合法组合靠单点维持（E9）；history isize 哨兵 + draft 双真值（E10）；last_action 字符串 37 写点（E11）；paste_burst 三谓词不一致（E12）；渲染脏标志簇（E13）；生命周期暂停旁路（E14）；settings 子菜单轮询关闭 + 字段双语义（E15）；panel pending_scroll 双真值（E16）；scroll_region 渲染前命中旧帧（E17）；tabs 跨函数握手（E18）；布局缓存副产物当输入（E20）；markdown 三元缓存不随 mermaid_mode 失效（E21）；loader 中止后仍转圈（E22）；补全源竞态无界索引（E23）；kitty 全局位双副本（E25）。

## What Changes（方向）

- 组件内状态类型化（显式模式枚举、单一缓存失效源、token 化动作标识）；渲染副产物与输入真值分离。

## Impact

- crates/xylitol-tui。引擎内部收敛，公共组件行为不变。

## Further Notes

- 与 c3285（指针/选区/焦点）同 crate 不同 seam，注意实施顺序。
