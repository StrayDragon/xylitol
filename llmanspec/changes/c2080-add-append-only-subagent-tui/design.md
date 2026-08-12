# Design: c2080-add-append-only-subagent-tui

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| 第三条产品面：**append-only**（观察/旁路；优先 sub-agent 轨迹） | Sub-Agent 编排 M1–M4（委托权限 / 合并 / 多面总览另票） |
| CLI `SurfaceMode` 再挂一条入口；经同一 `XyDriver` / `XyEvent` | 复活主会话「用户可切换 Inline↔AO」；改 ath30 主会话默认 |
| 库 `InteractionMode::Inline` 作载体（主屏 + emulator-owned 选区） | 主线 fold 族（activity-fold / 点三角 / Ctrl+O 展开故事） |
| session 旁统一 spill + 矮块硬帽（无折叠展开） | Web/TUI 公共减噪第二套故事；完整 slash/chrome 移植 |
| 架构探针：复用 / 故意不复用表可审查 + harness | AO 内嵌套子面板 / 第二进程编排（本票后置） |

## 产品一句话

主会话仍是 ath30 **ApplicationOwned-only**。本票交付一条**额外**瘦面：一味 append、无折叠交互、极简键、块可见上限严格小于主线，溢出进 **session 旁唯一 spill 目录**并在 transcript 用既有 `full_output_path` / `[Full output: …]` 族关联。顺带验证「多 app 模式、多 TUI 分别消费」接得住 `XyDriver` seam。

## 依赖与顺序

```text
c2071 archive ✅（ath30 AO-only 主会话）
  → c2080 Designed（本文件 + tasks；未 start）
  → change start（Branch binding）
  → Specs landing（新 capability + ath30 范围澄清）
  → apply（面 + harness + 架构审查）→ verify → archive
```

**硬闸**：`depends_on: c2071`（已归档）。编排 M1 **不**挡本票 start。

## 面入口（write-surface）

对齐 `write-surface` 流水线，**禁止**第二套 ReAct / 事件总线：

```text
composition::build_agent（或既有 driver 装配）
  → XyInProcessDriver : XyDriver
  → XyDriver::run → XyEvent 流
  → append-only host / bridge / 瘦 layout（面专属）
```

| 接线点 | 决策 |
|---|---|
| CLI 分发 | 扩展 `SurfaceMode`（今日仅 `Tui` / `Print`）→ 增加 **`AppendOnly`**；显式 flag / 子命令进入（非 TTY 默认、非抢主会话 TTY 默认） |
| 组合根 | 与 Print/TUI 同级再挂一条；仍只经 `app/core` + `XyDriver` |
| 目录意向 | 新产品面模块（与主 `app/tui` 并列或子树 `append_only`）；**禁止**把主线 AO host 叉成双模式 forever |
| 死代码 | apply 动手前对本面落点跑 `audit-dead-code`；禁止在未驱动骨架上堆 chrome |

## InteractionMode 选型

| 选项 | 结论 |
|---|---|
| **钉：库 `InteractionMode::Inline`** | 主屏差分 + emulator-owned 选区；无 alt-buffer / 无应用内选区 / 无 dock / 无 mouse capture 产品义务 |
| 不用 AO 小窗 | AO 绑定完整 chrome、dock、选区、fold hit——与「瘦旁路」目标相反 |
| 产品命名 | 对外 / specs：**append-only surface**（或 subagent-surface）；**禁止**教用户「切回 Inline 模式」；库 Inline 仅实现载体 |

正交声明（写入 specs 时）：**折叠策略 ⊥ InteractionMode**。本面「无折叠」是呈现策略，不是模式枚举第三值。

## Spill 契约（升格 `full_output_path`）

### 今日事实

- 工具累加器溢出 → `std::env::temp_dir()/xylitol-output-{uuid}.txt`，经 `full_output_path` + `[Full output: …]` 露出（`OutputAccumulator` / bash tool / TUI bridge）。
- **非** session 旁目录；散落系统 tmp。

### 本票目标

| 规则 | MUST |
|---|---|
| 根目录 | 每个 session **唯一** spill 目录，落在 session store 旁（与该 session id 关联；非全局散落 tmp） |
| 关联 | transcript / tool 结果继续用 `full_output_path`（或等价字段）指向该目录内文件；UI 文案保持 `[Full output: …]` 族 |
| 触发 | （a）工具硬截断（既有字节帽）；（b）本面块可见行帽溢出（即使未触达工具字节帽） |
| 展开 | **无** Ctrl+O / activity-fold / 点三角；全文只经 spill 路径 |
| 清理 | session 删除/过期策略跟随 session store（本票不另发明全局 GC）；实现时文档化 |

infra 侧路径从「系统 tmp」收到「session 旁」若证明必须改累加器 → **允许最小改动**；面不得 reach `infra::*`，经既有事件/结果字段消费路径。

## 块可见上限（严格小于主线）

主线 AO 现状（代码事实）：tools preview **5** 行、write body **10** 行，且可 Ctrl+O 展开（att14）；硬截断禁展开（att16）。

| 块类 | 主线（参考） | **append-only 钉** |
|---|---|---|
| tool / bash / assistant 正文 | 5 + 可展开 | **硬帽 3 行**；不可展开 |
| write / diff 类 | 10 / 12 + 可展开 | **硬帽 5 行**；不可展开 |
| 硬截断 `[Full output:` | 禁展开 + 路径 | 同；路径落 session spill |

数字进 live specs（Specs landing）；apply 用常量单测钉死「&lt; 主线对应帽」。

## 键位闭集（产品）

**闭集以外的产品快捷键 MUST NOT 注册**（含完整 slash 发现、fold、session tree、settings/plate）。

| 键 / 动作 | 语义 |
|---|---|
| Busy 时 Esc（或既有 abort 和弦） | `XyDriver::abort`；停父必停子的编排语义**不在本票**（仅本面所绑 runtime） |
| 退出 | `/exit` **或** 空闲 Ctrl+C 确认退出（二选一实现，specs 钉一种；推荐保留 `/exit` + 空闲双 Ctrl+C） |
| 滚动 | 终端原生 scrollback / PgUp·PgDn（Inline 载体）；**无**应用内选区拖动手势义务 |
| Spill | **无**专用「打开文件」产品键；路径已在 transcript；用户用终端/OS 打开 |
| 明确禁止 | Ctrl+O 展开、Alt+E、点三角、Command Plate、会话树双 Esc、steer/follow-up 完整 chrome（本面可不暴露插话条） |

若观察需要最小输入：可保留单行 editor 仅用于 `/exit` 与（可选）只读旁注；**不**移植主线 slash 目录。

## 复用 / 不复用（验收表）

| 复用（MUST） | 不复用 / 面专属（MUST 分叉） |
|---|---|
| `XyDriver` / `XyEvent` / `XyDriverError` | activity-fold / 点三角 / Ctrl+O 视口展开 |
| `composition::build_agent` 装配路径 | 主线 AO chrome（status/footer/cue/queue 全套） |
| session store + Trust 边界（不越父） | 完整产品键位表与 slash 发现 |
| 工具结果 → `full_output_path` / Full output 文案族 | AO 应用内选区 / mouse capture / dock |
| `RuntimePorts::materialize_runtime` 隔离约定（探针/后续接线） | 主线块默认 5/10 行 + 可展开 |
| 共享 tool 展示 helper（语义仍通时） | 主线 `HostSession` God 路径复制粘贴 |

**架构探针验收（与功能同权）**：审查 + harness 证明第二面未 fork ReAct；上表两侧均有至少一条自动化或清单勾选。

## ath30 / specs 策略

| 项 | 钉 |
|---|---|
| ath30 | **保持**主会话产品 TUI = ApplicationOwned-only；Specs landing 时**澄清主语**=「主会话产品 TUI」，不 silently 加例外削弱 AO |
| 新 capability | **`app-tui-append-only`**（host/transcript/键位/spill 预算；正式 req_id 在 landing 定） |
| CLI | 按需触 `cli-*` 或把入口行为写进新 capability（避免堆单体） |
| 编排可见性 | **另票**；本票 harness 用 `ScriptedDriver` / Fake 事件流即可 |

## 嵌套形态

| 阶段 | 钉 |
|---|---|
| **本票** | **独立产品入口**（`SurfaceMode::AppendOnly`）：单独 TTY 会话跑瘦 host，证明多模式接线 |
| **非本票** | 主 AO 内子面板 / 旁路嵌套；多 agent Agents Window；父杀子进程编排 |

独立入口是架构探针的最小可证伪单元；嵌套复用同一 host 实现可后续票吸收。

## 与 Sub-Agent roadmap

本票 = **面 + 架构探针**。roadmap M1（委托/权限/可见性产品规则）**另开 change**。本面命名与 research 对齐「旁路观察」，但不交付派生/合并语义。

## 验证

| 层 | 内容 |
|---|---|
| Specs | 新 capability + ath30 主语澄清；`validate --strict` |
| Harness | ScriptedDriver：提交→流式/工具→超帽 spill 路径可见→abort→/exit；**无** Ctrl+O 展开 |
| 架构审查 | 复用/不复用表；import 边界（无 `infra` / `capabilities` reach-in） |
| 人验 | 可选：TTY 下 `AppendOnly` 入口扫读 + spill 文件存在 |

## Open Questions（已钉）

见 `proposal.md` / `tasks.md` 同表。无未决项挡 start。

## Start readiness

| 项 | 状态 |
|---|---|
| `depends_on` c2071 已归档 | ✅ |
| proposal / design / tasks 齐 | ✅ |
| Open Questions 全钉 | ✅ |
| Branch binding / Specs / 应用代码 | ⛔ 未做（本阶段禁止） |
| **`ready_for_start`** | **`true`** — 可在干净默认分支执行 `llman sdd change start c2080-add-append-only-subagent-tui` |
