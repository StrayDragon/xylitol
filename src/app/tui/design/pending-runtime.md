---
version: "alpha"
name: "pending-runtime"
description: "NextTurn pending chrome — active in footer; Status trail on agent-busy row; revert/idle converge."
tokens_from: "../DESIGN.md"
components:
  pending-trail:
    textColor: "{colors.muted}"
---

# Pending runtime（NextTurn 挂账）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 产品意图：[`docs/roadmaps/运行时即时设置.md`](../../../../docs/roadmaps/运行时即时设置.md) M0。
> Status trail 槽：[`status.md`](./status.md)。Footer：[`footer.md`](./footer.md)。
> 静图：[`playground/`](./playground/)「Pending / NextTurn」槽 + Full shell 芯片。

## 产品意图

agent **还在跑**（含 in-flight 模型流）时用户换了模型 / thinking：

- **输入框下方 footer** 仍显示 **生效中**（当前 turn / in-flight 实际在用的值）；
- **agent-busy status 行右侧**（Status trail）dim 显示 **即将接替**；
- 再选回生效中 → trail **消失**，恢复如初；
- **不**靠 scrollback System 行解释「何时生效」。

idle / 无 in-flight：切换直接写入生效中（无 trail）；footer 更新；仍无成功 System 墙。

## 词汇

| 词 | 含义 |
|---|---|
| **选中（selected）** | 会话当前选定的模型 / thinking（`select_model` / cycle 后） |
| **生效中（active）** | 本 agent run **实际握住**的 in-flight / 本 turn 请求所用值（≠ 仅读 ModelManager） |
| **pending** | `selected ≠ active`；仅此时画 trail |

## MUST

1. **双态**：`pending` 时 MUST 同时可见 active（footer）与接替（trail）；MUST NOT 只把 footer 改成新模型而暗示当前流已换。
2. **Footer = active**：有 pending 时 footer 的 model / thinking MUST 反映 **active**；MUST NOT 用 selected 覆盖 footer 主字段。
3. **挂账 = Status trail**：仅 **agent run busy** 时，与 status **同一行**右侧（`.status-trail`）；左侧 **`.status-lead`** = `spinner + 短词` 一体、**MUST NOT** 把短词右对齐到行尾。见 [`status.md`](./status.md)。
4. **仅 bang busy**：MUST NOT 画 `Next turn:` trail；换模 / thinking MUST 按无 in-flight——footer 跟选中，无 pending 态。
5. **Trail 文案（M0 · 互斥优先级，非拼接）**：模型与 thinking **均只经 `/model` picker** 提交（无全局 Shift+Tab）。busy 内先后提交两次（或一次提交两者但仅模型变化 / 仅 level 变化）时，trail **只显示一项**（优先级：模型 > thinking）：

   | 情况 | trail |
   |---|---|
   | 模型 pending（无论 thinking 是否也 pending） | `Next turn: {model}` |
   | 仅 thinking pending（同模型改 level） | `Next turn thinking: {level}` |
   | 皆无 | （无 trail） |

   **MUST NOT** 使用 `Next turn: {model} · {level}` 拼接。等级文案仅为 **xylitol 档名**。窄宽优先截断 trail。
6. **清除 / 收敛**：
   - `selected == active`（含用户切回）→ 清 trail；
   - **同 run 下一 turn** 消费 → active←selected，清 trail；
   - **AgentEnd / abort 落定 / 进入 idle** 且无 in-flight → active 与 selected **收敛**，清 trail（补「末 turn 换模后无下一 turn」洞）。
7. **成功路径**：chrome 已表达时 MUST NOT 追加 muted System 确认行。
8. **与队列共存**：steer/follow-up 在 queue 带；trail 在 status 右；互不合并。
9. **静图 / fixture**：playground 三态——仅模型 / 仅 thinking / cleared；fixture `pending-model|thinking.next-turn`；`check_tui_design_playground` 绿。

## 形状（固定 · Status trail）

**恒定示例 active**：footer = `ornith · thinking off`。

### 仅模型 pending（`/model` 之后）

```
⠋ Working                    Next turn: deepseek-v4-flash
~/xylitol · ornith · thinking off
```

### 仅 thinking pending（`/model` 同模型改 level）

```
⠋ Working                    Next turn thinking: high
~/xylitol · ornith · thinking off
```

### 一次 Enter 提交 model+level（模型优先）

若模型与 level 相对 active 都变了 → trail **只** `Next turn: {model}`（level 一并在 NextTurn 消费，不拼接进 trail）。

### cleared / 切回 / idle 收敛

无 trail。

静图芯片：playground「Pending / NextTurn」→ **仅模型** / **仅 thinking** / **cleared**；Models 槽见 level 列。

## 防回归 / 反例（实现必测意识）

| 风险 | 要求 |
|---|---|
| active 误绑 ModelManager | footer/trail 以 **in-flight / run 握住的值** 为 active；选中单独存 |
| 末 turn 换模后无下一 turn | idle/abort **必须收敛**，禁止幽灵 pending |
| status 短词轮换丢 trail | trail 与 lead 短词解耦；compact/retry/tool 后仍在 |
| 窄宽涨行 | 截断 trail；status 内容仍一行 |
| 仅 bang 画 Next turn | 禁止；无 LLM 边界语义 |
| 多类 pending 炸槽 | 模型优先于 thinking；**禁止** `model · level` 拼接；能力覆盖不进 M0 |
| 差分闪烁 | trail 变更勿重置 spinner 相位；勿整壳无故重排 |
| System 墙回流 | 成功换模禁止 `model →` scrollback |

## 不做

- footer `a → b` 作为唯一挂账通道。
- queue 带再占一行 `Next turn:`。
- Settings / Plate；Scope `all|scoped`；能力覆盖进 M0 trail。
- 把 pending 写成 scrollback System 墙。

## 验收指针

- playground：三芯片；legend 标明 selected vs active；无拼接态。
- Full shell：换模挂账（模型态）+ agent busy；可与队列同开。
- 落地 harness：footer=active；agent busy 含 trail；切回 / idle 清 trail；status 轮换保 trail；bang-only 无 trail。
