---
depends_on: []
---

# TUI 形态注册表 + 代码驱动目录 + 帧带验证

> **一句话**：把 transcript 形态从闭世界 `match`/`_ => {}` 收成可注册贡献；场景走生产 paint；用帧带测流式关系。静图不再当第二套实现。
> **目的地**：加一种块 = 加 variant + 填表 + 加场景；编译器与 tape 拦住漏计/串态。

---

## Why

c1760–c1762 证明：ActivityFold 的词表、计数、live 身份是按「当前有哪些 `UiEntry` / 工具名」直接写成的。教训见 `.tmp/c1762-activity-fold-labels-lessons.md`：簇头 vs L1 和弦、Used 按次 vs 去重、全局 `streaming_thinking` 串改已封口 Thought、投影行（Todo）被算进 N。

这不是单票文案问题。`count_middles` 对未知变体 `_ => {}` 静默忽略；`is_edit_tool` 等是字符串表。新形态或减形态都容易漏。同时视觉 SSOT 叠了 DESIGN.md、`design/*.md`、2664 行手写 playground、specs `atc4`、以及真正的 paint——Agent 还被要求默认忽略 playground。截图无法表达帧关系。

## What Changes

- **形态贡献表**（产品 transcript，非插件市场）：每种块声明计数策略、簇头角色、是否投影、live 是否绑稳定 id。禁止吞掉新变体。
- **场景目录**：同一份 Scene/fixture 驱动生产 paint、语义断言、insta、可选生成静图。手写 `index.html` 降为生成物或退役。
- **帧带**：MockClock + 事件序列 → 断言第 N 帧语义树（哪簇 Thinking、哪簇必须仍是 Thought）。
- **性能**：密封簇 invalidate 范围可测；多开会话资源用 `lab_` 探针，默认 qa 不绑网。
- **Specs**：本票若改行为合约，只钉可观察折叠/计数语义，**不**钉模块路径。`atc4` / `adp*` 的「文档为 SSOT」与 B 线 `c2210` 联动，本票可 `depends_on` 或并行改措辞。

## Capabilities（意向）

| capability | 说明 |
|---|---|
| `app-tui-transcript` / fold 相关 | 形态开闭与诚实计数（产品可观察） |
| `package-tui-testing` | 帧带 / 场景目录（验证基础设施；避免写死文件行号） |
| `app-tui-design-playground` | 降级或改为「由场景生成」；与 c2210 一起瘦身 |

正式化时再拆 req。无合约变更的纯重构切片可 `skip_specs_landing`。

## 非目标

- 插件市场 / 运行时热加载第三方 widget
- 把 `agent_demo` 收成产品 SSOT
- 逐行滑入动画（已禁止）
- 本票清扫全部 65 个 spec（那是 c2210）

## Further Notes

调研：[`research/code-as-design-and-tui-verify.md`](./research/code-as-design-and-tui-verify.md)
追踪：`_HANDOFF/01-tui-kind-catalog-verify.md`
