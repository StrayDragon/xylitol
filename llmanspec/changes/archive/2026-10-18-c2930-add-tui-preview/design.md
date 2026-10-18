# 预览表面：TUI 内场景目录 + 假 Host

## 1. 形态

| | 端（怎么画） | Host（系统怎么应） | 入口 |
|---|---|---|---|
| 产品 TUI | TTY，同一套 client | 真 Host（attach 本机监听） | `xylitol` / `xylitol tui` |
| **预览** | TTY，同一套 client | 假 Host（磁带 / 注入） | `xylitol tui designing`（仅 debug 构建） |

预览不是第二种产品，不是网页里再实现一套格子。人看见的就是产品 TUI 组件。缺的是 **组合根换绑** 与 **场景槽**。designing 是 **交互表面的 debug 叶**，不是顶层表面。

三件分开说：

| 层 | 叫法 | 含义 |
|---|---|---|
| 看哪一帧 | **场景** | 一条命名可达态（树打开、busy、短终端…） |
| 系统怎么应 | **假 Host** | 不碰用户仓库 / 真 MCP / 真网关 |
| 人在哪切 | **场景槽** | 预览表面内的槽，列出并选场景 |

禁止叫：debug、playground、sandbox、检视工坊。harness 是按键→状态的测试机；预览会用到注入臂，预览本身不是 harness。

## 2. 为什么不用 `/debug`

1. **词表**：产品 slash 是用户学习成本；调试入口不该占 `/debug`。
2. **表面**：场景加载是预览组合的能力，不是 attach 真 Host 时的对话命令。
3. **目录**：Storybook 是侧栏选 story，不是在输入框打命令。TUI 对应物是 **槽**，不是 slash。
4. **隔离**：`/debug` 活在当前 TUI 进程的 slash 路径上，容易让人以为在改正在用的会话。预览必须显式换表面，并新建隔离会话。

废止后：产品命令模块对 `/debug`、`/debug <id>`、`/debug:<id>` 一律不识别（与未知斜杠相同）。预览表面也不得把 `/debug` 当作切场景的正道。

## 3. 假 Host

浏览器或预览进程都没有用户那台机器的仓库与 MCP stdio。预览要的保真度是 **UI / 交互 / 面状态机** 跑在真组件上；系统副作用用磁带。

| 口 | 产品 | 预览 |
|---|---|---|
| 工具 / bash | 真执行 | 磁带或空操作 |
| 模型 | 真网关 | 不调用，或注入已有 `XyEvent` |
| 会话 store | 用户 JSONL | 隔离的预览会话，不得写进用户当前 session 文件 |
| Trust / MCP | 本机项目 / 有配置才装配 | 不装配真连接；需要「已连接」那一帧就注入投影 |
| 对话条目 / 固定区 | 真事件 | 现有场景注入臂（Resume / LiveTape / LiveXy / 固定区操作） |

组合根换的是挂哪一个 Host，组件内不得 `if preview` 画第二套 UI。

注入保持 **进程内**：不进四象限信封，不进远程 unary。远程 TUI 连真 Host 时看不到预览场景表。

## 4. 场景槽（TUI 内）

预览表面打开后：

- 必须能打开场景槽（可替换 editor 槽，与会话树 / 模型列表同类，而不是常驻底栏 debug strip）。
- 槽内列出场景 id + 描述。
- 选中 → 关闭槽（或保留槽，画布已切换）→ 画布为该场景的真组件帧。
- 未知 id（若经测试/API 点名）→ 通知条或滚动提示短错误，不得崩溃。
- 产品 TUI（非预览表面）不得出现该槽，也不得注册场景补全。

键盘：预览表面内用与其它槽一致的打开/关闭（Esc 关槽）。本 change 不发明第二套键墙。

## 5. CLI

designing 挂在 **已经打开的交互表面** 下，这样以后 `xylitol desktop designing`（gpui 等）与 `xylitol tui designing` 同构。顶层 `xylitol preview` / `xylitol designing` 会把「设计中」做成第三种产品面，下一张桌面端无法复用同一叶。

本 change 只钉 TUI：

- debug 构建：`xylitol tui designing` 打开预览组合（假 Host + 场景槽）。`xylitol tui --help` 列出 `designing`。
- release 构建：`xylitol tui --help` **不得** 列出 `designing`；`xylitol tui designing` 按未知子命令失败。
- `xylitol --help` 的 Commands **不得** 含 `preview` 或 `designing`（含 debug 构建）。
- 不得提供 `tui --designing` 旗标（一种入口；旗标会和 attach / port 搅在同一 flatten）。
- 不得放进 `serve` 子树。
- TTY 裸跑 **不得** 进入预览。
- 预览 **不得** 因本机 Host 未在听而失败（r1391 只约束产品 TUI）。

## 6. 与后续「眼睛」的关系（本 change 不做）

TUI 只有一层单元格，inspect 框不能画进 ANSI。以后若要人/agent 对同一帧指认组件，需要格子层与 overlay 层分离（原生 dump → 薄 HTML 框 → 可选 wasm 切片）。那是下一张合约：`InspectFrame`（格子 + 命名框目录）。本 change 只让 **场景在真 TUI 里可切、可看**。没有命名框，预览已经能给人眼和 insta 用。

后续 change 宜依赖本表面已经存在的场景表与假 Host，而不是再从 `/debug` 长出来。

## 7. 明确不做（实施期）

| 不做 | 原因 |
|---|---|
| ANSI 里画 inspect 框 | 改用户所见，和 ApplicationOwned 选区 / hover 打架 |
| 预览里真 exec 用户仓库 | 保真度错；用磁带 |
| 产品默认入口改预览 | 预览是设计/开发协作 |
| xstate 解释器进运行时 | 图论门禁只看转移表 |
| 为预览新建网页组件树 | 禁止第二套 UI |
