---
depends_on: []
---

# 第二产品面：gpui 桌面客户端（Linux/Wayland）

产品面格局收敛为 **gpui 桌面 + TUI 双 Rust 面**（2026-08-22 拍板，2026-09-24 复核仍维持，见 `docs/research/ui-runtime-tradeoffs-2026.md`）。本票交付 gpui 面的第一可交付切片：attach 本机 Host（四象限信封）的只读观察者。方向正文：`docs/roadmaps/Gpui桌面客户端.md`。

## Why

用户优先级（性能占用 / 内存极致 / 原生 computer use）下，浏览器系形态自败；gpui 与 Hyprland 主线同栈且 Rust 直用协议闭集。Host 信封与多 session 已就绪，缺的只是新 client——不动 ReAct / 工具主线。

## What Changes（草案，propose 时拍板）

- 新桌面应用入口（独立 binary 或 feature）：经四象限信封 attach 本机 Host（同产品 TUI 拓扑），M1 只读渲染 transcript 与状态头卡。
- 复用 protocol 类型闭集直连 serde，无跨语言类型闸负担；MUST NOT 发明第二套会话/命令词表。
- 跨面同源：公共能力动作语义与 TUI 一致（约束板 `跨端同源.md`）；GUI 增强（点击/悬浮）仅为面内附加。
- 本机集成（剪贴板/编辑器/通知）按 client 职责留面本地，M3 再议。

## 开放决策（propose 时深挖）

- 独立 workspace 成员（如 `packages/xylitol-gpui`）vs 主 crate feature——依赖树隔离与编译时间权衡。
- gpui 上游未到 1.0：版本钉死策略、vendored 与否、破坏性漂移跟进成本上限。
- 渲染投影源：冷恢复走 get_messages 快照（w8 语义）、实况走 mux 订阅——与 TUI 同一套 driver/client 缝还是平行实现。
- M2 对话回路的键位映射表与 TUI 同构程度。

## Capabilities（拟）

- 新 capability（命名落地时定，倾向 `app-gpui-*` 族）；跨面同源条款挂既有约束板。

## Impact

新增一个可选面；TUI/Print/host 行为零变化。内存增量 ~百 MB 级（GPU 直绘），换取终端外的原生体验与 Computer Use 呈现层。

## 非目标

Windows/macOS；Web 控制台 / SPA；远程多工作区管控；在桌面面发明第二套会话词表；本票内做对话回路（M2 另票）。

## 状态注记

Draft（方向壳）。真实动工信号：用户认领 M1 切片 → propose 完整路径（Branch binding + Specs landing）。gpui 上游 API 漂移是本票最大外部风险，propose 时先做 spike 评估。
