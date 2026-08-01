# xylitol-tui 能力钩子审计（对照景观 §8 · 2026）

> **用途**：承接 [`coding-agent-tui-design-landscape-2026.md`](./coding-agent-tui-design-landscape-2026.md) §8，盘点本仓引擎包相对「通用 TUI 能力钩子」的已有 / 部分 / 缺口。
> **范围**：`packages/xylitol-tui` 为主；`src/app/tui` / infra 仅作接线对照。
> **非目标**：不定实现计划、不改 live specs、不对照 pi 视觉雷同。

## 1. 一句话结论

引擎核（差分 **inline** 渲染、`Component`/overlay、`ExpandableOutput`、扁平 `KeybindingsManager`）已可用；§8 里偏 **agent 体验** 的钩子（语义复制、装饰档位、鼠标选区、分层 keymap、footer 插件槽、双轨 buffer）多数 **未进包**，由产品/infra 部分承接或仍缺口。

## 2. 对照表

| # | 钩子 | 判定 | 层级 | 证据（摘要） |
|---|------|------|------|----------------|
| 1 | 渲染双轨 scrollback ↔ alt buffer | **部分** | 包 | `TUI::differential_render` / `finish_inline` / `with_terminal_suspended`（`tui.rs`）：固定 inline + 退出留 scrollback；**无** EnterAlternateScreen / 运行时模式切换 |
| 2 | 语义复制 `copy_as_markdown(turn_id)` | **缺口** | 产品弱替代 | `/history-copy-last` → `XyDriver::copy_text_to_clipboard`：末条助手纯文本；无 turn_id / MD / history 寻址。包无复制 API |
| 3 | Scrollback 交还 `dump_to_scrollback` | **缺口** | 产品弱替代 | `/session` stats dump ≠ transcript 展开；无 `expand_tools` 一次性 dump API |
| 4 | 装饰档位 `raw \| compact \| rich` | **缺口** | 邻近碎片 | `GlyphSet`、`ExpandableOutputOptions`、`DiffOptions::compact_*` 分散；无统一档位、不影响选区 |
| 5 | 鼠标/选区策略 | **缺口** | — | 无 `EnableMouse` / copyOnSelect / shift passthrough；`tui.input.copy` 键位有定义、Editor 未接线 |
| 6 | 上下文化 keymap + leader | **部分** | 包+产品 | 扁平 `tui.*`/`app.*` + overlay 焦点；**无** composer/transcript/vim/approval 分层或 leader |
| 7 | Footer/statusline 插件槽 | **部分** | 产品 | `format_footer_text` 固定字段序；无有序插件槽 / footer 交互持久化 |
| 8 | Tool 折叠摘要 + 展开 | **部分** | 包 primitive + 产品编排 | 包 `ExpandableOutput`；产品 `ScrollbackFold` / 三态 bg / Alt+E·Ctrl+O |
| 9 | 剪贴板 native + OSC52 + `/dev/tty` | **部分** | infra+产品 | `plan_clipboard_copy` / `format_osc52` / host emit；**无** `/dev/tty` fallback；**不进包** |
| 10 | 外部编辑器 / export | **部分** | 包钩 + 产品/agent | 包 `with_terminal_suspended`；产品 Ctrl+G `$EDITOR`；`/session-export` HTML/JSONL，**无** markdown export |

## 3. 包 vs 产品 vs demo

| 能力 | 包 | 产品 | `agent_demo` |
|------|----|------|--------------|
| 差分 inline | ✅ | host 驱动 | ✅ |
| Expandable primitive | ✅ | 完整块语义 | ✅ 先行 |
| 键位目录 + JSON | ✅ 引擎 | ✅ `app.*` | 部分 |
| Footer / transcript 壳 | — | ✅ | 简化 |
| 剪贴板 / export | — | ✅ Driver 缝 | 部分 |
| vim / leader / mouse / 装饰档位 | — | — | — |

**升级引擎仍算缺口（若 remaster 要「库级」）**：2、3、4、5、7（插件槽）、可选 1（真双轨）、9（若要求包级 clipboard）、10（markdown export 偏 agent 域）。

## 4. HTML playground 该模拟什么

| 适合静图（视觉档位） | 必须 demo / Rust |
|----------------------|------------------|
| `raw \| compact \| rich` 同屏对照 | 差分渲染残影、宽高调屏 |
| footer 元数据下沉 vs 消息徽章墙 | overlay eligible/blocked/resume |
| tool 一行折叠 / 展开块 | OSC52 / 真剪贴板 |
| busy + 下轮预告、chrome toast | `$EDITOR` suspend |
| dark/light、GlyphSet 样张 | keymap 热重载与冲突 |
| 复制清洁示意（「选区会带什么」旁注） | 鼠标/终端选区协议（未实现） |

## 5. 进包 vs 留宿主（建议，未定案）

| 倾向进包（通用） | 倾向宿主 / agent |
|------------------|------------------|
| 装饰档位影响绘制的原语开关 | turn 寻址语义复制、session export 格式 |
| 鼠标 capture / passthrough 策略 | `/history-copy-*` 产品命令与文案 |
| keymap context / leader 引擎支持 | footer 业务字段语义（model/token） |
| （可选）alt-buffer 模式切换 | transcript dump 的业务展开规则 |

## 参考

- 景观：[`coding-agent-tui-design-landscape-2026.md`](./coding-agent-tui-design-landscape-2026.md)
- 包边界：`packages/xylitol-tui/AGENTS.md` · `PI_DELTAS.md`
- 产品面：`src/app/tui/AGENTS.md` · `design/AGENTS.md`
