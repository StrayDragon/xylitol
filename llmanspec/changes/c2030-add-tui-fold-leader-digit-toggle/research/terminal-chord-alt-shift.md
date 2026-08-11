# Research: 终端对 Alt/Shift/Ctrl 和弦的送达 vs 自吞

> Change: `c2030-add-tui-fold-leader-digit-toggle`
> 范围：Kitty / Ghostty / WezTerm / Foot（含 footclient）
> 方法：一手文档 / man / 官方默认绑定源；不跑交互探测。

对齐仓内：`c1760` 已写「`Ctrl+Alt+Shift+E`：**库支持、终端有条件**；现代端通常能报三修饰+字母；传统 VT/tmux/远程不可靠；MUST 可配」——本文**不推翻**该结论，只补四端默认绑定与协议细节。见 [`../c1760-add-tui-activity-fold/proposal.md`](../c1760-add-tui-activity-fold/proposal.md)「`Ctrl+Alt+Shift+E` 支持吗？」；同目录 [`fold-leader-vs-global-alt-e.md`](fold-leader-vs-global-alt-e.md) / [`fold-leader-seams-code-facts.md`](fold-leader-seams-code-facts.md)。`docs/research/` 无同题耐久稿。

xylitol-tui 启动推 Kitty flags `>7u`（1+2+4）：`packages/xylitol-tui/src/terminal.rs`。

---

## 1. 协议事实（跨端）

Kitty keyboard protocol 编码独立修饰位：`shift=1, alt=2, ctrl=4`（值 = `1 + bits`）。无协议时，文档自陈：**难可靠使用「除 shift+alt / ctrl+alt 以外」的多修饰组合**；开启 **Disambiguate（flag 1）** 后，`Esc` / `alt+key` / `ctrl+key` / `ctrl+alt+key` / **`shift+alt+key`** 改走 `CSI u`。三修饰+字母（如 `ctrl+alt+shift+e`）属 legacy 不可靠区，依赖 `CSI u` / Kitty 增强或 modifyOtherKeys。

- https://sw.kovidgoyal.net/kitty/keyboard-protocol/

---

## 2. 分终端表（默认配置下）

图例：**达 app?** = 默认未绑终端 UI 时，raw/协议下通常能当作键事件送达（用户可改绑推翻）。

### Kitty

| Chord | 达 app? | 已知默认冲突 | Notes |
|---|---|---|---|
| Alt+E | 是 | 无 | legacy≈`ESC e`；flag1→`CSI u` |
| Alt+Shift+E | 是 | 无 | flag1 明确含 `shift+alt+key`→`CSI u` |
| Ctrl+Alt+Shift+E | 有条件 | 无默认绑 | 需协议/`CSI u`；本端原生支持 |
| Ctrl+T | 是 | 无 | |
| Ctrl+Shift+T | **否** | **new tab** | `kitty_mod` 默认=ctrl+shift |
| Ctrl+O | 是 | 无 | |
| （邻接）Ctrl+Shift+E | **否** | **open_url_with_hints** | 不是 Alt+Shift+E |
| （邻接）Ctrl+Shift+Alt+T | **否** | set tab title | |

来源：https://sw.kovidgoyal.net/kitty/overview/ · https://sw.kovidgoyal.net/kitty/actions/ · https://sw.kovidgoyal.net/kitty/mapping/

### Ghostty

| Chord | 达 app? | 已知默认冲突 | Notes |
|---|---|---|---|
| Alt+E | 是 | 无 | 绑键默认 **consume**，不编码进 PTY |
| Alt+Shift+E | 是 | 无 | |
| Ctrl+Alt+Shift+E | 有条件 | 无默认绑 | 官方宣称支持 Kitty keyboard protocol |
| Ctrl+T | 是 | 无 | |
| Ctrl+Shift+T | **否**（Linux/非 Darwin） | **new_tab** | macOS 默认多为 Super+T |
| Ctrl+O | 是 | 无 | |
| （邻接）Ctrl+Shift+E | **否**（Linux） | **new_split:down** | |
| （邻接）Ctrl+Shift+O | **否**（Linux） | **new_split:right** | |
| （邻接）Alt+1…8 | **否**（Linux） | **goto_tab** | leader 用「Alt+E 后再按数字」OK；勿默认 `Alt+digit` |

来源：https://ghostty.org/docs/config/keybind · https://ghostty.org/docs/features · 默认表 `src/config/Config.zig`（`ghostty-org/ghostty`）· `ghostty +list-keybinds --default`

### WezTerm

| Chord | 达 app? | 已知默认冲突 | Notes |
|---|---|---|---|
| Alt+E | 是 | 无（默认表无 Alt+字母） | Alt+Enter=fullscreen |
| Alt+Shift+E | 是 | 无 | |
| Ctrl+Alt+Shift+E | **更有条件** | 无默认绑 | **`enable_kitty_keyboard` 默认 false**；否则多靠 xterm/modifyOtherKeys |
| Ctrl+T | 是 | 无 | |
| Ctrl+Shift+T | **否** | **SpawnTab** | Super+T 亦 new tab |
| Ctrl+O | 是 | 无 | |
| （邻接）Ctrl+Shift+Alt+"/%/方向 | 否 | pane split/resize | 非字母 E |

来源：https://wezterm.org/config/default-keys.html · https://wezterm.org/config/key-encoding.html · https://wezterm.org/config/lua/config/enable_kitty_keyboard.html

### Foot / footclient

| Chord | 达 app? | 已知默认冲突 | Notes |
|---|---|---|---|
| Alt+E | 是 | 无 | 默认 meta→ESC 前缀（`foot(1)` ALT/META） |
| Alt+Shift+E | 是 | 无 | |
| Ctrl+Alt+Shift+E | 有条件 | 无默认绑 | `foot-ctlseqs(7)`：Kitty `CSI ?/>/</= … u` + modifyOtherKeys L1 默认 / L2 可开 |
| Ctrl+T | 是 | 无 | |
| Ctrl+Shift+T | 是* | *默认无 tab 动作 | 新窗是 **Ctrl+Shift+N**；用户可自绑 T |
| Ctrl+O | 是 | 无 | |
| （邻接）Ctrl+Shift+O | **否** | **show-urls-launch** | |

**footclient**：与 foot 共用同一套 `[key-bindings]` / 协议栈（server 模式 `app-id` 默认 `footclient`）；无第二套和弦表。

来源：https://man.archlinux.org/man/foot.1 · https://man.archlinux.org/man/extra/foot/foot.ini.5.en · https://man.archlinux.org/man/extra/foot/foot-ctlseqs.7.en

---

## 3. 显式 callout：`Alt+Shift+E` / `Ctrl+Alt+Shift+E` 是否常被偷？

| Chord | 四端默认是否常偷？ |
|---|---|
| **Alt+Shift+E** | **否**——四端官方默认表均未见占用。 |
| **Ctrl+Alt+Shift+E** | **默认绑定：否**；**送达：有条件**（对齐 c1760）。风险在协议/多路复用器，不在「终端 UI 默认抢键」。 |
| 易混近邻 | Kitty **Ctrl+Shift+E**=URL hints；Ghostty Linux **Ctrl+Shift+E**=竖分屏——旁注勿写成 Ctrl+Shift+E。 |

---

## 4. 对 xylitol fold-leader 的实用建议

1. **保留 c1760**：`Alt+Shift+E` / `Ctrl+Alt+Shift+E` 作段栈默认合理；四端不抢这两键；三修饰 MUST 可配 + 弱端文档承认（与 c1760 一致）。
2. **c2030 leader**：优先 **`Alt+E` → 数字**（数字在 app 模式内消费）。四端默认不抢 `Alt+E`；**避开默认 `Alt+1…8`**（Ghostty Linux 切 tab）。
3. **勿默认依赖**：`Ctrl+Shift+T`（Kitty/Ghostty/WezTerm 新 tab）；亦勿把产品键写成易混的 `Ctrl+Shift+E/O`。
4. **既有产品键**：`Ctrl+T` / `Ctrl+O` 在四端默认表上干净——可继续作 thinking / tools-output；与 fold-leader 错开。
5. **WezTerm**：文档提示用户开 `enable_kitty_keyboard = true`（或接受 modifyOtherKeys），否则三修饰收纳更脆。
6. **与 [`fold-leader-vs-global-alt-e.md`](fold-leader-vs-global-alt-e.md)**：方案 D 用 `Alt+Shift+E` 当全局块折会 **直接撞 c1760**——仍禁止。

---

## 5. 来源速查

| 端 | URL / man |
|---|---|
| Kitty protocol | https://sw.kovidgoyal.net/kitty/keyboard-protocol/ |
| Kitty shortcuts | https://sw.kovidgoyal.net/kitty/overview/ · https://sw.kovidgoyal.net/kitty/actions/ |
| Ghostty keybind | https://ghostty.org/docs/config/keybind · Features VT |
| WezTerm keys / encoding | https://wezterm.org/config/default-keys.html · key-encoding · enable_kitty_keyboard |
| Foot | `foot(1)` · `foot.ini(5)` · `foot-ctlseqs(7)` |
| xylitol | c1760 proposal；`packages/xylitol-tui/src/terminal.rs` |
