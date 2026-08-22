# Gpui 桌面客户端

> 第二产品面：gpui 桌面应用（**Linux / Wayland 先行**），与 TUI 并列的双 Rust 面格局。
> 吃同一 Host（attach 本机监听器，四象限信封），不另起第二套会话/工具故事。
> 方向拍板：2026-08-22（前端选型收敛，Web 面出局；见 [`../research/ui-runtime-tradeoffs-2026.md`](../research/ui-runtime-tradeoffs-2026.md)）。

## 用户怎么碰到

| 场景 | 痛点 | gpui 面要给的 |
|---|---|---|
| 终端之外的日常 | TUI 依赖终端模拟器的键位/剪贴板/渲染脾气 | 原生窗口：自有键位、选区、字体与滚动 |
| 内存克制的桌面应用 | Electron 类动辄数百 MB | GPU 直绘的原生 Rust 面，增量 ~百 MB 级 |
| Computer Use（Hyprland）联动 | 桌面操作缺呈现层 | 与 Computer Use 主线同栈：Linux/Wayland 原生优先 |
| 多面并存 | 换面丢上下文 | 同一 session 经信封 attach，跨面可对上 |

## 产品边界

| 要 | 不要 |
|---|---|
| Linux + Wayland 先行做深 | 早期铺 Windows/macOS |
| 复用 Host 全部语义（会话/队列/中止/trust/MCP） | 在桌面面发明第二套会话或工具词表 |
| 与 TUI 同源：公共能力一套学习成本 | 把面专属交互（点击/悬浮）写成公共 MUST |
| 单机 attach 本机 Host | 远程多工作区管控（那是搁置的 Cloud-Agent） |

## 与 TUI 的关系

跨面同源约束继续适用（约束板：[`Web与TUI同源.md`](./Web与TUI同源.md)——标题历史遗留，实质是**跨面**同源板）：两面描述同一会话时状态可对上；公共能力的动作语义同源、快捷键尽量同构；仅面专属能力（纯 TTY / 纯 GUI 增强）可分叉。两 Rust 面共享协议类型闭集，无跨语言类型闸负担。

## BDD 意图示例

**场景：双面观察一致**
Given 同一 session 分别被 TUI 与 gpui 面 attach
When 一轮对话完成
Then 两面对同一 transcript 与队列状态的投影可对上

**场景：面专属增强不越界**
Given gpui 面提供点击折叠等 GUI 增强
When 用户在 TUI 学习了公共动作
Then 公共动作语义一致，增强仅为面内附加

## 分阶段

| 阶段 | 用户可感知结果 |
|---|---|
| M1 观察者 | attach 某 session，只读渲染 transcript 与状态头卡 |
| M2 对话回路 | 提交 / steer / follow-up / abort 全链可用 |
| M3 本机集成 | 系统剪贴板、外部编辑器、通知走面本地能力（client 职责） |
| M4 Computer Use 呈现 | 桌面控制进度/授权在 GUI 呈现（依赖 Computer Use 主线 M2+） |

## 依赖与并行

- Host 信封与多 session 已落地；本方向主要是**新 client**，不动 ReAct / 工具主线。
- 与 Computer Use 主线天然同栈（Wayland），呈现层协同但不互相阻塞。
- 插件形态（未来 wasm 组件 / lua）住 host 侧，一套服务所有面；本方向避免引入浏览器运行时导致扩展体系分裂。

## 支线与方向

- 字体/主题系统与 TUI 主题的映射关系（跨面视觉词汇，后议）。
- gpui 上游 API 未到 1.0：跟进策略与 vendored 固化点在实现提案时定。
