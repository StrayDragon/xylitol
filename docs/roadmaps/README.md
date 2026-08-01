# 产品路线图（与 pi 分道后的候补方向）

> **只写尚未兑现的高维产品方向。** 已兑现心智只在 [`docs/architecture/`](../architecture/README.md)。
> **闭环规则** → [`docs/AGENTS.md`](../AGENTS.md)。本目录是**统一优先级的候补板**，不是进度表。
> 某篇全部兑现后：**删除该文件**并更新本索引，不留占位。

现状对齐：2026-08-01。追溯归档 change：`llman sdd archive freeze --list`；产品文尽量不钉 change id。

## 闭环

```text
docs/roadmaps/  →  llmanspec/changes  →  docs/architecture/
     ↑                                    │
     └──── 兑现后删除/收缩本文档 ←─────────┘
```

## 怎么用

| 写 | 不写 |
|---|---|
| 未兑现的用户可感知方向、依赖、BDD 意图 | 已落地 MUST（那是 architecture） |
| 可并行主线、**分阶段切片**、支线意向 | 进度勾选、状态列、「已迁入」对照表 |

认领时：读各篇「分阶段」表，一次只提案**一个可交付切片**（通常 Mn）；有 MUST/SHALL → SDD；纯主题/文案 → quick。
支线表 = 头脑风暴候补，**不**等于已排期。

## 候补一览

```mermaid
flowchart TB
  subgraph Surfaces["应用面"]
    Homo["Web 与 TUI 同源 · 约束板"]
    Web["Cloud Agent 与 Web 控制台"]
    Obs["OTEL 与 Langfuse 观测"]
    Visual["TUI 视觉与信息表达"]
    Remaster["TUI 重制"]
    Keys["键位与命令发现"]
  end

  subgraph SessionCtrl["会话内控制"]
    Live["运行时即时设置"]
    Loop["Loop 管理与触发可视化"]
    Sub["Sub-Agent 编排"]
  end

  subgraph Depth["深度能力 · 零成本默认"]
    Lsp["LSP 会话集成"]
    Dap["DAP 调试集成"]
    Multi["多模态理解"]
    Cu["Computer Use · Hyprland"]
    Prov["预设 Providers"]
  end

  subgraph VerifyOpt["可验证与省成本"]
    Eval["Agent Eval 与回归基准"]
    Cache["上下文缓存与极致压缩"]
  end

  Homo -.->|跨面约束| Web
  Homo -.->|覆盖事实源| Live
  Homo -.->|公共发现路径| Keys
  Visual -.->|减噪与密度| Remaster
  Web -.->|学习键盘挂靠| Keys
  Live --> Lsp
  Live --> Dap
  Live --> Sub
  Live --> Loop
  Obs -.->|排障对照网关| Prov
  Obs -.->|Dataset 种子| Eval
  Sub -.-> Web
  Cache -.->|策略变更须回归| Eval
  Cache -.->|工具摘要协同| Lsp
  Cache -.->|工具摘要协同| Dap
  Prov -.->|cache 能力声明| Cache
```

> **观测**：本地 fastrace JSONL + OTLP/Langfuse 已落地（见 [architecture](../architecture/进程内观测.md)）；本文只留子进程出站候补。**不**自研 Inspect 检视台。
> **压缩**：会话 auto-compact + provenance 已落地（见 [architecture](../architecture/压缩与上下文.md)）；缓存/动态压缩见新篇。
> **Eval 调研底稿**：[../research/agent-eval-frameworks-2026.md](../research/agent-eval-frameworks-2026.md)。
> **键位学习 UI 调研**：[../research/keybinding-keyboard-visualizer-2026.md](../research/keybinding-keyboard-visualizer-2026.md)（挂 [键位与命令发现.md](./键位与命令发现.md) M4）。
> **TUI 重制**：景观 [../research/coding-agent-tui-design-landscape-2026.md](../research/coding-agent-tui-design-landscape-2026.md)；引擎缺口 [../research/xylitol-tui-capability-hooks-vs-landscape-2026.md](../research/xylitol-tui-capability-hooks-vs-landscape-2026.md)。

| 文档 | 候补方向 |
|---|---|
| [Web与TUI同源.md](./Web与TUI同源.md) | 跨面语义约束板（含长历史 activity 折叠栈意向；非 Web 壳本身） |
| [TUI视觉与信息表达.md](./TUI视觉与信息表达.md) | 状态减噪、主题密度；与 activity 折叠减噪衔接 |
| [TUI重制.md](./TUI重制.md) | **UiEntry** rail 皮肤；独立 remaster 页；无洗底 + 条目空行 |
| [键位与命令发现.md](./键位与命令发现.md) | busy 即时列表（含 resume 只读）；`/hotkeys` 支线延后；可视化键盘挂 Web；busy 列表顶掉 status → `c1810`（调研在 change design） |
| [OTEL与Langfuse观测.md](./OTEL与Langfuse观测.md) | 子进程出站观测（已落地见 [architecture](../architecture/进程内观测.md)） |
| [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md) | SWE 先、TB 后；Docker 出分；AA 选模对照；Langfuse 回归旁路 |
| [上下文缓存与极致压缩.md](./上下文缓存与极致压缩.md) | prompt cache、动态压缩、工具结果分级压缩 |
| [Cloud-Agent与Web控制台.md](./Cloud-Agent与Web控制台.md) | 多工作区 CS + Web |
| [运行时即时设置.md](./运行时即时设置.md) | 能力覆盖盘 / 可观察覆盖集 / Web 同源（模型 NextTurn 已迁 [architecture](../architecture/运行时即时设置.md)） |
| [Loop管理与触发可视化.md](./Loop管理与触发可视化.md) | Loop 管理与触发醒目 |
| [Sub-Agent编排.md](./Sub-Agent编排.md) | 子 agent 派生/回收/可见 |
| [LSP会话集成.md](./LSP会话集成.md) | lspz；会话启停；零成本 |
| [DAP调试集成.md](./DAP调试集成.md) | dapz；调试集成；零成本 |
| [多模态理解.md](./多模态理解.md) | 音频 / 视频 + 模型档案模态声明 |
| [Computer-Use与Hyprland.md](./Computer-Use与Hyprland.md) | Linux/Hyprland computer use |
| [预设Providers.md](./预设Providers.md) | 命名预设厂商档案 |
