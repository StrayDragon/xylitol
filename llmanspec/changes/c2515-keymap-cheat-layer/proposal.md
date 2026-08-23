---
depends_on: []
---

# 键位速查层：让「还有哪些键可用」随时可见

## Why

产品键位持续增长（tree 过滤族、tools 族、session 族……），但发现方式只有
`keybindings.json` 文档与记忆。现有 keybindings manager 已持有全部定义、
冲突检测与反查 API，缺的是一个**展示层**：按下前缀后浮出可用键位速查，
或随手唤出全量速查面板。

## What Changes

- 新增速查浮层：按上下文列出当前可用的 action → 键位映射
  （数据全部来自 keybindings manager 反查，文案零手工维护）。
- 两种形态待拍板：
  a) **前缀等待态**：引入 leader 前缀机制（如单前缀键 + 单字母命令），
     前缀按下未完成时浮出候选；
  b) **即时面板**：一个快捷键唤出全量/分域速查（只读，无前缀机制）。
- 浮层渲染走包层 overlay 通道；Esc 关闭；不影响焦点恢复合约（c575 已有）。

## 非目标

- 不改任何现有键位绑定；不引入强制前缀（a 形态若采纳也是可选层）；
- 不做键位自定义 UI（`keybindings.json` 热重载维持现状）。

## Impact

- 包层 overlay/keybindings 小扩展；app 层一个浮层组件；
- harness 补「浮层打开时不吞业务键 / Esc 还原」切片。

## Further Notes

- 现状核对（manager API 清单）与两形态对比：[research/keymap-cheat-notes.md](./research/keymap-cheat-notes.md)
