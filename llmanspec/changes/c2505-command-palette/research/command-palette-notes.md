# 命令面板 调研补充（c2505）

> 2026-08-23。结论先行：数据与执行通道全部现成，唯一硬约束是
> **Ctrl+P 历史预留给已冻结的 Plate stub**——入口键位需要显式拍板。

## 现状核对（代码事实）

- 命令目录 SSOT：`app::product_commands`（`src/app/mod.rs` / `src/app/core/mod.rs`），
  运行时经 `XyDriver::get_commands` 组装（含 MCP 动态命令）；app/tui 只读消费。
- 执行路径：slash 解析 → pending → effects 泵 → Driver，**单一通道**
  （`src/app/tui/AGENTS.md`：禁止第二套 slash/steer 执行路径）。
- 键位设施：keybindings manager 支持 `extend_definitions` 注册新 action +
  `get_keys` 反查展示（`packages/xylitol-tui/src/keybindings.rs:200-256`）。
- **冻结约束**：面 AGENTS 明文「不做运行时 Settings/Plate 改配置；
  不绑定 Ctrl+P 打开 Command Plate stub」。本票是「命令面板」不是「Plate」，
  但入口若用 Ctrl+P 即触碰该条，必须先修订决议。

## 参照实现手法

成熟实现的命令面板共性：

1. 面板项 = 命令 + 设置项/视图动作混排，未输入时置顶「建议」分组；
2. footer 显示命令当前绑定键（从 keymap 引擎反查，不手工维护文案）；
3. 选中即执行的语义与 slash 直输完全一致（同一命令 ID 分发）。

## 引入设计（规格草案）

- 数据：`get_commands()` 全量 + 每命令元组 `(名称, 描述, 别名?, 绑定键?)`；
  过滤 = c2500 的纯函数模糊匹配。
- 视图：c2500 协议渲染；选中行右侧显示绑定键。
- 执行：产出等价 slash 字符串走既有 pending 路径（零新执行语义）。
- 入口：候选 a) Ctrl+P（需解冻并同步 AGENTS） b) `/` 空触发即弹
  （现 slash catalog 已接近此形态，差异仅在模糊过滤与全量范围） c) 新 leader 前缀。

## 决策点

1. 入口键位（涉及 Plate 冻结决议是否解除）；
2. 范围：仅产品命令 vs 含视图动作（折叠、主题等本地动作）——后者超出
   product_commands SSOT，需要 app 层第二注册表，建议二期。

## 影响面

- 一个 slot + keybindings 注册；无协议变更；BDD 一条等价性场景。
