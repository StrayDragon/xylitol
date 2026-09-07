# xylitol-tui 能力钩子审计（对照景观 §8 · 2026）

> **用途**：承接 [`coding-agent-tui-design-landscape-2026.md`](./coding-agent-tui-design-landscape-2026.md) §8，盘点本仓引擎包相对「通用 TUI 能力钩子」的已有 / 部分 / 缺口。
> **范围**：`packages/xylitol-tui` 为主；`src/app/tui` / infra 仅作接线对照。
> **非目标**：不定实现计划、不改 live specs、不对照 pi 视觉雷同。
> **时效**：能力快照（§1–§3 及证据行）截至 2026-09-06，c2020/c2070/c2071 之后按代码刷新；§4–§5 为叙事 / 建议底稿，不随代码漂移。

## 1. 一句话结论

引擎核（差分 **inline** 渲染、`Component`/overlay、`ExpandableOutput`、扁平 `KeybindingsManager`）已可用；**交互双入口**（Inline + ApplicationOwned 鼠标选区）已随 c2020/c2070/c2071 进包并由产品缺省承接；§8 里偏 **agent 体验** 的钩子（语义复制、装饰档位、分层 keymap、footer 插件槽、双轨运行时热切）多数 **未进包**，由产品/infra 部分承接或仍缺口。

## 2. 对照表

| # | 钩子 | 判定 | 层级 | 证据（摘要） |
|---|------|------|------|----------------|
| 1 | 渲染双轨 scrollback ↔ alt buffer | **部分** | 包 | 差分 inline（`render_frame`/`render_now`、`finish_inline`、`with_terminal_suspended`）+ `InteractionMode::ApplicationOwned`（`begin_application_owned_session`：EnterAlternateScreen + mouse capture，`terminal.rs`）；AO 退出回写主屏 scrollback；仍无 mid-session 模式热切（构造期绑定） |
| 2 | 语义复制 `copy_as_markdown(turn_id)` | **缺口** | 产品弱替代 | `/history-copy-last` → `XyDriver::copy_text_to_clipboard`：末条助手纯文本；无 turn_id / MD / history 寻址。包无**语义**复制 API（仅 AO 选区 OSC52） |
| 3 | Scrollback 交还 `dump_to_scrollback` | **部分** | 包 | `finish_application_owned` 默认把会话 transcript（+ 末帧 dock）回写**主屏** scrollback（`set_append_session_to_main_scrollback_on_exit`）；仍无 mid-session / `expand_tools` 寻址 dump |
| 4 | 装饰档位 `raw \| compact \| rich` | **缺口** | 邻近碎片 | `GlyphSet`、`ExpandableOutputOptions`、`DiffOptions::compact_*` 分散；无统一档位、不影响选区 |
| 5 | 鼠标/选区策略 | **部分** | 包+产品 | 包 `InteractionMode::ApplicationOwned`（alt + mouse capture）+ `SelectionController` copy-on-release 默认开；产品缺省 ApplicationOwned（ath30），dock 排除输入面、折叠命中 `set_transcript_hit_priority`；仍无 shift passthrough，`tui.input.copy` 键位仍无键盘接线 |
| 6 | 上下文化 keymap + leader | **部分** | 包+产品 | 扁平 `tui.*`/`app.*` + overlay 焦点；**无** composer/transcript/vim/approval 分层或 leader |
| 7 | Footer/statusline 插件槽 | **部分** | 产品 | `format_footer_text` 固定字段序；无有序插件槽 / footer 交互持久化 |
| 8 | Tool 折叠摘要 + 展开 | **部分** | 包 primitive + 产品编排 | 包 `ExpandableOutput`；产品 `ScrollbackFold` / 三态 bg / Alt+E·Ctrl+O |
| 9 | 剪贴板 native + OSC52 + `/dev/tty` | **部分** | 包+infra+产品 | OSC52 已进包：`format_osc52` + copy-on-release（transcript / Editor 出 `pending_clipboard`，脱离差分批量 flush）；native 剪贴板仍 infra `plan_clipboard_copy` + host emit；仍无 `/dev/tty` fallback |
| 10 | 外部编辑器 / export | **部分** | 包钩 + 产品/agent | 包 `with_terminal_suspended`；产品 Ctrl+G `$EDITOR`；`/session-export` HTML/JSONL，**无** markdown export |

## 3. 包 vs 产品 vs demo

| 能力 | 包 | 产品 | `agent_demo` |
|------|----|------|--------------|
| 差分 inline | ✅ | host 驱动 | ✅ |
| Expandable primitive | ✅ | 完整块语义 | ✅ 先行 |
| 键位目录 + JSON | ✅ 引擎 | ✅ `app.*` | 部分 |
| Footer / transcript 壳 | — | ✅ | 简化 |
| 剪贴板 / export | OSC52 松手复制 | ✅ Driver 缝 + native | 部分 |
| vim / leader / 装饰档位 | — | — | — |
| 鼠标选区（ApplicationOwned） | ✅ | ✅ 缺省 | ✅ `agent_demo_alt` |

**升级引擎仍算缺口（若 remaster 要「库级」）**：2、4、7（插件槽）、可选 1（运行时热切）、10（markdown export 偏 agent 域）；3（退出回写）、5（鼠标选区）、9（OSC52 包级剪贴板）已进包。

## 4. HTML playground 该模拟什么

| 适合静图（视觉档位） | 必须 demo / Rust |
|----------------------|------------------|
| `raw \| compact \| rich` 同屏对照 | 差分渲染残影、宽高调屏 |
| footer 元数据下沉 vs 消息徽章墙 | overlay eligible/blocked/resume |
| tool 一行折叠 / 展开块 | OSC52 / 真剪贴板 |
| busy + 下轮预告、chrome toast | `$EDITOR` suspend |
| dark/light、GlyphSet 样张 | keymap 热重载与冲突 |
| 复制清洁示意（「选区会带什么」旁注） | 鼠标/终端选区协议（ApplicationOwned 已落地，demo 需 AO 模式） |

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
