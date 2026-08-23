# 键位速查层 调研补充（c2515）

> 2026-08-23。结论先行：manager 的数据面（定义/反查/冲突）完备，
> 缺的是**展示层**；两形态差异在于是否引入「前缀等待」这一新机制。

## 现状核对（代码事实）

- `packages/xylitol-tui/src/keybindings.rs`：
  - `matches_event`(:200) / `get_keys`(:211) / `get_definition`(:215)
  - `get_conflicts`(:219) 冲突检测已有
  - `set_user_bindings`(:223) 热重载（c1090）、`extend_definitions`(:256) 注册新 action
  - 作用域：`KeybindingsScope`（thread-local Rc<RefCell<Manager>>，
    enter/with/with_keybindings_mut）；产品 HostSession 经 scope 持有
- 键事件模型：crossterm 解码**完整和弦**（如 Ctrl+Alt+Shift+E 单事件到达），
  引擎内**不存在**部分匹配 / 前缀等待状态机（rg 验证无 leader 概念）
- overlay 设施：包层 `OverlayHandle` + eligible/blocked/resume（c575），
  焦点恢复合约现成

## 两形态对比

| | a) 前缀等待态（leader 机制） | b) 即时速查面板 |
|---|---|---|
| 新机制 | 需在 dispatch 路径引入「和弦序列」状态机（首键挂起、超时回放） | 无——纯展示浮层 |
| 能力 | 单字母命令命名空间解锁，键位扩展不再挤双修饰键 | 发现性补强，不改键位结构 |
| 风险 | 与 crossterm 完整和弦模型摩擦最小，但挂起期吞键策略需谨慎设计；Esc 语义要并入现有 Esc 分岔规则 | 几乎无风险 |
| 参照 | 成熟 agent TUI 常见 leader + which-key 组合 | 各类 TUI 的 bindings help 面板 |

## 引入设计（规格草案）

- **b 先行**：一个只读速查浮层（overlay 通道打开），按域分组列出
  当前 scope 全部 action → keys（`get_definition/get_keys` 直出），
  支持按域过滤（tree/tools/session/editor…），Esc 关闭。
- **a 为可选二期**：若采纳 leader，前缀键本身注册进 definitions；
  挂起期浮出候选（which-key 式），超时/无效键回放并恢复原语义。
- 浮层文案零手工维护（全部反查）；Ascii 字形集兼容。

## 决策点

1. 形态拍板：仅 b / b+a 二期；
2. 若做 a：leader 前缀键选型与挂起超时时长；
3. 浮层分域粒度（全局/当前槽上下文两级？）。

## 影响面

- b：app 层浮层组件 + 包层 overlay 复用，无引擎改动；
- a（若采纳）：包层 dispatch 增加序列状态机——触及硬约束「输入硬切」边界，
  MUST 走 propose 全流程评审。
