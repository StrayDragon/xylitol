# UI 运行时选型决策：gpui 桌面 + TUI 双 Rust 面（2026-08）

> **性质**：跨 change 主题级耐久底稿——记录 2026-08-22 前端选型收敛的依据、被拒项与复活条件。产品方向正文在 [`../roadmaps/Gpui桌面客户端.md`](../roadmaps/Gpui桌面客户端.md)；本文只管「为什么这么选、什么条件下翻案」。

## 决策

1. 第二产品面 = **gpui 桌面（Linux / Wayland 先行）**，与 TUI 并列双 Rust 面。
2. **Web TS 面出局**：c2310（薄 TS 客户端）draft 删除；Cloud-Agent 控制台 roadmap 搁置；TS 类型导出链路（specta bindings / gen-sdk / pa-bind1 drift 闸）已于 2026-08 移除——协议类型只以 Rust `protocol` 为真源，非 Rust 消费者需求出现时再重立。
3. **wasm 的定位是未来插件沙箱（WIT 组件），不是 UI 运行时**。

## 优先级输入（用户给定）

性能占用 → **内存极致少** → 原生 computer use（Hyprland/Wayland）→ 插件形态（wasm/lua）需可容纳。

## 架构前提

Host/client 已分离：模型/会话/MCP/trust 在 host（serve 或同进程），面只是四象限信封 client。因此：

- 内存大头在 host；UI 只贡献增量。
- computer use 的 OS 访问在 host 工具侧（wlroots/grim/wtype 类），与 UI 框架无关。
- UI 选型决定的是呈现质量、本机集成面（剪贴板/编辑器/通知/全局键）、以及将来「面的扩展」归谁管。

## 对比结论

| 轴 | gpui | wasm-Rust UI（yew/dioxus/leptos）| Web TS |
|---|---|---|---|
| 内存增量 | +100~200MB（GPU 直绘） | 浏览器 tab 100~300MB + wasm 线性内存 | tab 150~400MB |
| computer use | 强：原生窗口/a11y/截屏注入全可达，与 Hyprland 主线同栈 | 弱：沙箱禁输入注入与截屏，须另配 native companion（形态自败） | 弱（同沙箱）；管控台场景本不需要 |
| 类型 | Rust 直 `use` 协议闭集 | protocol 编译 wasm target 可复用，但 UI 框架生态 churn 大 | 需自建 TS 类型层（导出链路已移除，重立项时重建） |
| 插件协同 | host 一套插件服务所有面 ✓ | 浏览器 UI 会诱导 JS/wasm 第二套 UI 扩展体系 → 双轨分裂 | 同左 |
| 维护 | 重依赖树 + 上游 API 未到 1.0（随 Zed 漂） | 三框架快变，押注风险高 | 双语言栈 |

## 插件形态备注

- mlua：~1MB 级依赖、嵌入浅、可 sandbox、冷启快——个人 agent hook/工具脚本性价比最高。
- wasmtime + WIT 组件：隔离最强、行业标准方向，但依赖与内存重一档。
- 插件住 host，与 UI 选型正交；native-first 保证未来只有一套扩展系统。按扩展开闭纪律，未立项前不预挖。

## 翻案条件（满足其一才重议）

- 出现真实非 Rust 消费者需求（自研 Web 面重立项或第三方集成合同级需求）→ 重开 Web TS 链路（c2310 思路可复刻）。
- gpui 在 Linux/Wayland 的维护成本失控（上游破坏性漂移无法跟）→ 重评 egui/iced/slint 等 Rust 原生备选，仍不回浏览器。
- 多工作区远程管控成为真实个人需求 → Cloud-Agent 底稿复活，届时 Web 是管控台而非编码主面。

## 相关落点

- 方向正文：`docs/roadmaps/Gpui桌面客户端.md`
- 同源约束板（实质跨面）：`docs/roadmaps/Web与TUI同源.md`
- 搁置愿景：`docs/roadmaps/Cloud-Agent与Web控制台.md`
