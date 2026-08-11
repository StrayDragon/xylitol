# Web 与 TUI 同源

> **约束板**：凡已开闸的 Web 与 TUI，对会话 / 改道 / 能力开关必须**语义同源**；表现可异。
> Print / TUI / Server 同构已在 architecture；本文**不**交付 Web 壳本身（见 Cloud-Agent）。
> 现状对齐：2026-07-22。
> 观测出口统一走 fastrace → 可选 OTLP/Langfuse（见 [OTEL与Langfuse观测.md](./OTEL与Langfuse观测.md)），**不**另起「检视事实源」产品面。

## 本文是什么 / 不是什么

| 是 | 不是 |
|---|---|
| 跨面语义 MUST 与可验证意图 | Web 控制台 / 多工作区产品交付 |
| 给即时设置、编排等挂「同源」约束 | 要求像素或键位一致 |
| 可在无完整 Web 壳时先钉共享会话语义 | 假装今日已有 GUI/Web 面 |

Web 应用面交付 → [Cloud-Agent与Web控制台.md](./Cloud-Agent与Web控制台.md)。

## 同源 vs 同貌

| 同源（MUST） | 同貌（不要求） |
|---|---|
| 驱动命令与事件闭集 | 像素布局、信息密度、控件皮肤 |
| 会话 / 队列 / 中止 / 分支 | — |
| 即时设置覆盖集；Sub-agent / Loop 关键状态 | — |
| **公共能力**的动作语义与学习模型（用户「要学什么」一致） | 某一面**独有**能力的交互 |
| 公共能力的快捷键 / 发现路径 **SHOULD 尽量同构**（OS 修饰键 ⌘/Ctrl 等可映射） | 强制每个物理键帽字节级相同；禁止 Web 在同构键之外再提供点击等增强 |

> **分界**：先问「这是不是两面都会有的能力？」——是 → 语义与理解成本必须一套，键位尽量一套；否（纯 TTY / 纯 DOM）→ 可分叉，但仍勿污染公共词汇。

## 产品规则

| MUST | 禁止 |
|---|---|
| 两面描述同一会话时状态可对上 | 静默分叉第二套会话语义 |
| 能力开闭（即时设置）共用事实 | Web 已关、TUI 仍当已开 |
| 新开闸面接入时对齐全套同源词汇 | 为赶 Web UI 另起一套进度故事 |
| 设计/改 **公共**交互时，按「一套学习成本」验收（含折叠类减噪、改道等） | 只为 TUI（或只为 Web）发明第二套公共故事；把面专属键位写成公共 MUST |

## 分阶段（可认领切片）

| 阶段 | 用户可感知 / 可验证结果 | 备注 |
|---|---|---|
| **M1 同源词汇与事实源清单** | 会话 / 改道 / 覆盖集的跨面约定可审查；公共能力「学习成本一套」写入可审查清单 | **文档+合约意图**；不要求 Web UI |
| **M1b 长历史 activity 折叠栈** | 公共能力切片示例：两面共用「从输入侧剥开 / 盖上」；动作 id 同源；键位 SHOULD 同构（TUI 默认可配） | 草案 `c1760` / `c1755`；**未兑现**；≠ session-compact；服从上文「公共 vs 专属」 |
| **M2 即时设置事实源** | TUI 覆盖集可被未来 Web 只读/写入同一语义 | 与 [运行时即时设置.md](./运行时即时设置.md) M1–M2 对齐 |
| **M3 跨面接续** | 同会话在已开闸的 TUI 与 Web 间切换不矛盾 | **依赖** Web 面至少有会话壳 |

## 依赖与并行

- **被依赖**：即时设置、Sub-agent、Loop、Cloud-Agent 的跨面叙事。
- **不阻塞**：Tokenizer CLI、TUI 视觉减噪、多模态档案声明（可先 TUI/CLI）、OTLP/Langfuse 出口。
- 现行多 client：[../architecture/库与多客户端.md](../architecture/库与多客户端.md)。

## BDD 意图示例

**场景：跨面接续**
Given 用户在 TUI 进行中的会话
When 在 Web 打开同一会话
Then 进度、队列与最近工具结果可理解且不矛盾

**场景：即时设置同源**
Given 用户在 Web 关闭了某后置能力覆盖（如 DAP）
When 回到 TUI 开启下一波次
Then 覆盖一致，且下一波次按关闭该能力装配

## 支线与方向

| 支线 | 意向 |
|---|---|
| **同源契约测试意图** | 跨面状态对拍的 BDD 意图清单（可先无 Web UI，钉词汇） |
| **覆盖集只读投影** | Web 未开写时仍能只读看见 TUI 覆盖（M2 轻量） |
| **事件闭集变更检查单** | 新生命周期词必须双面登记（流程向，非进度板） |
| **Eval harness 面无关** | 回归跑 Print/CLI，不要求 TUI/Web 像素同源 |
| **压缩/cache 状态同源** | 两面看到的「正在压缩 / cache 未知」语义一致 |
| **Activity 折叠栈（expandNearest / collapseNearest）** | 旧 turn 中间操作 → L2/L3 摘要；从输入侧双向栈；与 `/session-compact` 分层；TUI：[`c1760`](../../llmanspec/changes/c1760-add-tui-activity-fold/proposal.md)（**延后**；交互前置 [`c2070`](../../llmanspec/changes/archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)） |
| **TUI 交互 oneof（终端选区 ↔ 应用内选区）** | **ApplicationOwned**≈Pi `fullscreen`/alt-screen（**产品缺省 ath30** / [`c2071`](../../llmanspec/changes/c2071-update-app-tui-host-mode-b-only/proposal.md)；库 MUST：拖选、跨页续选、松手复制）；**Inline**≈Pi `regular`/主屏（库 lab/demo）。入口：[`c2070`](../../llmanspec/changes/archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)；术语→代码见同 change `research/emulator-vs-app-selection-oneof.md` §1。**c2020** 鼠标管道为 ApplicationOwned 地基；`XYLITOL_TUI_MOUSE` 仅 lab/e2e，产品不读。 |

## 相关

- [Cloud-Agent与Web控制台.md](./Cloud-Agent与Web控制台.md)
- [运行时即时设置.md](./运行时即时设置.md)
- [OTEL与Langfuse观测.md](./OTEL与Langfuse观测.md)
- [上下文缓存与极致压缩.md](./上下文缓存与极致压缩.md)
- [TUI视觉与信息表达.md](./TUI视觉与信息表达.md)（减噪切片与 activity 折叠衔接）
- Activity 折叠草案：[`c1760`](../../llmanspec/changes/c1760-add-tui-activity-fold/proposal.md)（**延后**；前置 [`c2070`](../../llmanspec/changes/archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)）
- 双交互架构顶层：`llmanspec/changes/archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes`
- 总索引：[README.md](./README.md)
