# TUI 输入延迟：排查方法论与结论

> **性质**：跨 change 耐久底稿。源自滚轮跟手性专项（2026-10，八版迭代收敛）——**方法论与工具长期有效；具体数字与结论句已抽象**，现行实现以代码为 SSOT（滚轮：`src/app/tui/mod.rs` wheel 分支 + `crates/xylitol-tui` AO runtime）。

## 结论（恒真版）

1. **终端 TUI 滚动的顺滑 = 恒定小步长直跳 + 纪律性合帧，无动画层**。pi（alt-screen）、zellij、opentui 三家对标均如此：数据搬移 → should_render → 合帧收割。**一切「聪明」（加速度曲线/缓动弹簧/预测惯性）在此规模都是负资产**——每层引入自身时序噪声，叠加事件脉冲即「顿挫」。若未来重引动画，必须先有帧间隔客观签名（见下）证明静态跳变不足。
2. **coalesce 是双刃剑**：合并高频事件省 CPU，但**主动抓取（drain now_or_never）会把连吃事件并成单帧巨跳**——「串内巨跳 + 串间空转」正是顿挫签名。事件粒度 paint 成本微秒级时，**逐事件即渲**优于批合并。
3. **输入双通道**：tmux 等复用层会把滚轮本地译成方向键（应用 capture 协商失败/未开时）；SGR 与键序列必须同等支持，否则「用户 A 顺、用户 B 卡」。

## 方法论四律（本次八版教训的抽象）

| 律 | 内容 | 反例（本项目实际踩过） |
|---|---|---|
| **仪器先于药方** | 体感性能问题先建客观签名（延迟分布/帧计数/字节量），改前改后同秤 | 盲改八版 accel/queue/弹簧，每版「好些/怪」循环；签名落地后一轮定位 |
| **E2E 必须覆盖真输入链** | mock 直调函数的单测**系统性漏掉**集成层（事件流唤醒/批处理/tty 协商）；真 PTY 注字节才算数 | 全部 lab 直调 `apply_*`，产品内 coalesce-burst 从未现形；e2e 落地当轮即抓到 |
| **先删后加** | 性能调优优先**减法**（删缓存层/删合并/删抽象）；加机制前先证明删无可删 | 弹簧+队列+accel 三层叠出，最终全部撤除回归直跳 |
| **观测随功能落** | lag/metrics 钩子与逻辑同 commit；事后补测量的成本与失真都更高 | paint 钩子姗姗来迟，前期全靠体感描述 |

## 工具地图（现状，代码为 SSOT）

| 工具 | 用途 | 入口 |
|---|---|---|
| wheel 延迟签名 E2E | 真 PTY 注 SGR，量首响/排空/帧数——「卡」的客观回归界 | `tests/tui_e2e/pty.rs::pty_product_wheel_latency_signature`（`just test-tui-e2e-pty`） |
| AO 性能 lab | busy 混合/滚轮帧成本/输出字节量 | `src/app/tui/lab_ao_perf.rs`（`lab_ao_busy_mixed_perf` 等，`#[ignore]`） |
| lag/wheel 观测 | 产品内 paint 耗时 + wheel delta 序列（顿挫 signatures） | `XYLITOL_DEBUG=1 RUST_LOG=xylitol::lag=info,xylitol::wheel=debug` → `just obs-tui-lag` |

## 排查 SOP（下次遇「卡」直接走）

1. **产品内**：`host_wheel_paint` 分布——高→渲染内容成本（缓存/wrap 向）；低→进 2
2. **事件侧**：`xylitol::wheel` delta 序列——出现 ±10 以上巨值=合并过度；gap 长尾=到达节拍问题
3. **真链**：wheel 签名 e2e——首响 p50 劣化=host 外（EventStream/pty）；正常=回 1/2 找产品内
4. **外部**：以上全清白→终端模拟器渲染器差异（录帧对比，换终端 cross-check）

## 关联

- 滚动交互产品语义（gutter/选中/follow）：`crates/xylitol-tui/llmanspec/specs/` 与 `src/app/tui/AGENTS.md`
- 观测窄读操作：`xylitol-dev` skill（`references/observability.md`）
