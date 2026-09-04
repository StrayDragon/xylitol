# 键位 JSON → 可交互学习键盘（调研 2026-08）

> **用途**：支撑 roadmap「键位与命令发现」**M4 延后切片**；**不是**产品 MUST，**本波不实现**。
> **输入假设**：xylitol 风格目录 ≈ VS Code 心智的「命令 id → 和弦列表」，磁盘为 `keybindings.json`：`{ "app.interrupt": ["escape"], "app.tools.expand": ["ctrl+o"] }`（值也可为单字符串）。与 VS Code 的 `key`/`command`/`when` 数组形不同，但可做一层 adapter。

## 1. 一句话结论

**学习键盘应挂 Web 控制台（或独立静态页），不要塞进 TTY。** 生态里几乎没有「喂一份 keybinding JSON → 自动得到可点选 QWERTY + 模拟按键」的成品；更接近的是 **cheat sheet / omnibar / kbd 徽章**（`use-kbd`、`react-hotkey-display`、`hotkey-hint`）。完整键盘图多半要 **自研薄层**（布局 SVG/组件 + 自有 chord 解析），或二次开发 heatmap/打字练习壳。Rust 内嵌 WebView（`wry`）技术可行，但对个人 TUI coding harness **过重**，且与现有 host 循环冲突；等 Web 面再做同源学习页更划算。

## 2. xylitol 现状（输入侧）

| 资产 | 说明 |
|---|---|
| 包默认 `tui.*` | `packages/xylitol-tui/src/keybindings.rs` |
| 产品 `app.*` | `src/app/tui/keybindings.rs` |
| 用户覆盖 | `~/.xylitol/keybindings.json`；`/reload` |
| 冲突 API | `KeybindingsManager::get_conflicts()` — 仅用户同键多 id；未进 UI |
| 发现 | 学键主路径 = 文档 / 日后 Web；产品不默认 `/help`；若程序内目录则倾向 `/hotkeys`（roadmap 支线延后）；demo 有 `/help` 墙 + Ctrl+P plate（产品禁 plate） |

上下文多义（树开/关同键不同 id）**不在 JSON 里**，学习 UI 必须带 **mode / when** 维度，否则会误导。

## 3. 前端库对照（二次开发候选）

| 库 | 形态 | 贴近「JSON→交互键盘」？ | 备注 |
|---|---|---|---|
| **[use-kbd](https://github.com/runsascoded/use-kbd)** | React：ShortcutsModal、LookupModal、Omnibar、可编辑绑定、JSON import/export、冲突检测、modes | **最接近产品层**（查/改/学），但 **不是** 全尺寸 QWERTY 键盘图 | 适合 Web 控制台「键位设置 + ? 帮助」；学习键盘可在其上加一层物理布局 |
| **[react-hotkey-display](https://github.com/mulkatz/react-hotkey-display)** | Kbd 徽章、Cheatsheet、轻量 Command Palette | 展示强；无物理键盘 | ~4KB；适合旁注与 cheatsheet |
| **[hotkey-hint](https://www.npmjs.com/package/hotkey-hint)** | `?` overlay、分组、序列 | 展示 + 注册运行时热键 | 零依赖；与 xylitol 运行时键路由无关时只拿 UI |
| **[@tanstack/react-hotkeys](https://tanstack.com/hotkeys)** | 注册/作用域/devtools | 运行时层；有 display 工具 | 勿与 TUI `KeybindingsManager` 双源 |
| **[@keybindy/react](https://github.com/keybindyjs/react)** | 作用域绑定 + ShortcutLabel | 标签级 | 薄 |
| **keyboard-heatmap / 打字练习壳** | QWERTY SVG 热力 | 有物理键盘，**无** command-id 绑定语义 | 可偷布局 SVG，自接 chord→高亮 |
| shadcn `Kbd` / Cladd Shortcut | 纯展示原子 | 否 | 设计系统零件 |

**推荐组合（若日后做 M4）**

1. **数据**：Rust/CLI 导出「resolved catalog」JSON（id、description、chords、**context/mode**）——比直接喂用户覆盖文件更真。
2. **壳**：Web 用 `use-kbd` 或自研 cheatsheet（查键、分组、模拟按下高亮列表项）。
3. **物理键盘**：自研或 fork heatmap 布局；点击键帽 → filter catalog；按下真实键 → 高亮键帽 + 列出该 mode 下命中的 id。
4. **VS Code JSON adapter**（可选）：若用户想从 VS Code `keybindings.json` 迁移，写 `key`/`command`/`when` → xylitol id 的映射表；**不要**假装 1:1。

## 4. 「内嵌到 Rust」选项

| 方案 | 可行性 | 与 xylitol 契合 |
|---|---|---|
| **Web 控制台内嵌页**（Cloud-Agent roadmap） | 高 | **首选**：同源学习成本；不碰 TTY |
| **CLI 打开系统浏览器** + 本地 `file://` 或 `localhost` 静态页 | 中高 | Web 壳前的薄过渡；零 WebView 依赖 |
| **`wry` / Tauri WebView 子窗** | 技术可行 | Linux 要 WebKitGTK + 事件环；与 crossterm TUI 双环痛苦；包体积/依赖重 |
| **TTY 内 ASCII 键盘组件** | 可行但丑 | 可做 M1 Help 的增强，**达不到**「标准键盘学习」体验 |
| **egui / iced 原生 GUI** | 可行 | 又一面；与「Web 同源」叙事分流，不优先 |

**结论**：不建议「TUI 进程内嵌 React 键盘」。等 Web 模式；过渡期最多「导出 JSON + 浏览器打开静态学习页」。

## 5. 可绑 id 速查（调研附录 · 易腐）

> 以代码为准；此处仅便于路线讨论。生成导出后应废弃手维表。

### 产品 `app.*`（`src/app/tui/keybindings.rs`）

| id | 默认和弦 | 说明 |
|---|---|---|
| `app.interrupt` | escape | Cancel / abort |
| `app.clear` | ctrl+c | 清空 / 退出 |
| `app.message.followUp` | alt+enter | 排队 follow-up |
| `app.message.dequeue` | alt+up | 还原队列 |
| `app.editor.external` | ctrl+g | `$EDITOR` |
| `app.thinking.toggle` | ctrl+t | thinking（**树关**） |
| `app.tools.expand` | ctrl+o | 工具视口（**树关**） |
| `app.tools.blocks` | alt+e | 块展开 |
| `app.tree.filter.*` | ctrl+d/t/u/l/a、ctrl+o、ctrl+shift+o | **树开** filter |
| `app.session.fork` | shift+f | fork |
| `app.tree.editLabel` / `toggleLabelTimestamp` | shift+l / shift+t | 标注 |
| `app.session.toggleSort|NamedFilter|Path|Id|rename|delete` | ctrl+s/n/p/u/r/d | **Resume 面板** |
| `app.paste.image` | ctrl+v | 剪贴板图 |

### 包 `tui.*`（节选）

编辑器移动/删除/yank、`tui.input.submit|newLine|tab`、`tui.select.*`（pageUp/Down **默认无键**）、`tui.tree.foldOrUp|unfoldOrDown|editLabel|toggleLabelTimestamp`。

### 建议改键（仅投诉驱动；默认不改）

| 现象 | 建议 | 优先级 |
|---|---|---|
| Ctrl+O / Ctrl+T 树开/关双义 | Help 分组说清；或树 filter 改到未占用修饰组合 | 低（习惯 + 旁注已在） |
| Ctrl+P = Resume path，demo plate 易混 | 文档写清；Plate 重开须换和弦 | 中（若重开 Plate） |
| Ctrl+U 三角色（editor / 树 / Resume） | 同左 | 低 |

## 6. SDD 入口（暂不立项）

| 切片 | 是否须 SDD | 说明 |
|---|---|---|
| M0 busy 列表闸 | **是** | 改 MUST 行为 |
| M1 `/hotkeys`（非 `/help`） | **延后支线**；用户明确要再 SDD | 学键主路径是文档 |
| M4 可视化键盘 | Web 表现为主时可能 **docs + quick**；若 TUI 新 slash/槽则 SDD | 本波只调研 |

认领 M0 时：`llman-sdd-explore` → `propose`，勿在默认分支改 live specs。

## 7. 相关

- Roadmap：[`../roadmaps/键位与命令发现.md`](../roadmaps/键位与命令发现.md)
- 设计合约：组件键写在 `designing/tui/modules/` 各模块 `draft.yaml` 的 `keys:`（无独立快捷键设计）
- Web：`../roadmaps/Cloud-Agent与Web控制台.md`、`../roadmaps/跨面同源.md`
