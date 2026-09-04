# Tasks: c2550-rename-terms-add-flow-view

测试 seam：全部复用既有——BDD harness（`cargo test --test bdd`，feature GWT 文案与 step 注册文案成对）、cargo 单测/harness（`just test`）、designing 闸（`scripts/check_tui_designing.py` 进 `just qa`）、playground 类型检查（`bun run check`）。不发明新 seam。

## T1 词表 SSOT 改名 [独立]

- [x] `docs/architecture/TUI信息面与固定区词汇.md` → `git mv` 为 `TUI信息面与固定区词汇.md`；标题与头部改「固定区」；关键词汇表：壳层通告行改**通知条**（toast notice · `ToastNotice` · `push_toast_notice`）、尾随行改**尾插**（tail-append）、新增**固定区**（fixed zone）行；E 类落点/原则/代码对应表同步；弃用表新增三行（壳层通告→通知条、尾随→尾插、chrome→固定区）
- [x] 验证：`rg -n "壳层通告|尾随|chrome" docs/architecture/TUI信息面*词汇.md` 仅剩弃用表与代码对应表中的映射行

## T2 讨论层词同步（docs / AGENTS / designing 文案）[blocked-by: T1]

- [x] 指针层：根 `AGENTS.md`、`docs/AGENTS.md`、`docs/architecture/README.md`、`docs/architecture/术语表.md`、`src/app/tui/AGENTS.md`、`packages/xylitol-tui/AGENTS.md`（仅指针行）、`designing/AGENTS.md` —— 词表文件名链接 + 词形替换
- [x] 派生 docs：`用户可见事件.md`、`运行时即时设置.md`、`扩展能力-MCP.md`、`用户可见事件`外其余 rg 命中 docs、`docs/roadmaps/键位与命令发现.md`（含 chrome-toast 模块路径链接 → toast-notice）
- [x] designing 全树：`chrome-toast/` → `git mv toast-notice/`（draft id / states id 同步）；各 draft/intent/states 中 壳层通告→通知条、尾随→尾插、chrome（讨论语义）→固定区
- [x] `just gen-designing-index` 重生成

## T3 代码标识符改名（src/，机械 + 逐处辨义）[blocked-by: T1]

- [x] toast 家族：`push_chrome_toast`/`chrome_toast*`/`render_chrome_toast_slot`/`expire_chrome_toast_now`/`clear_chrome_toast*` → `*toast_notice*`（含 harness/tests）
- [x] fixed_zone 家族：`ChromeOp`/`PreviewInject::Chrome`/`apply_chrome_op`/`SlotChromeRows`/`chrome_footprint*`/`sync_runtime_chrome`/`set_active_chrome`/`refresh_chrome_caches`/`chrome_hint`/`reserved_lower_chrome` → `*fixed_zone*`（文件 `layout/chrome_footprint.rs`→`fixed_zone_footprint.rs`、`layout/root/chrome_footprint_apply.rs`→`fixed_zone_footprint_apply.rs`、`effects/slash/chrome.rs`→`misc.rs`）
- [x] 中文注释与 doc 内 壳层通告→通知条、chrome→固定区、尾随→尾插
- [x] 验证：`cargo test`（workspace）全绿；`rg "chrome" src/` 零命中
- [x] packages/xylitol-tui 不动（fork）

## T4 specs 词同步 + capability 改名（Specs landing，在绑定分支先做）[blocked-by: T1]

- [x] `app-tui-chrome/` → `git mv app-tui-fixed-zone/`（文件、header、`功能:` 行）；场景 id：chrome-success-no-system→fixed-zone-success-no-system、chrome-toast-ephemeral→toast-notice-ephemeral、chrome-footprint-term-budget→fixed-zone-footprint-term-budget、chrome-no-extra-undocumented→fixed-zone-no-extra-undocumented
- [x] 其余 12 个 feature：壳层通告→通知条、chrome toast→toast notice、Chrome Footprint→Fixed-Zone Footprint、chrome（讨论语义）→固定区、尾随→尾插、场景 id 内 chrome→fixed-zone；`app-tui.feature:22` capability 引用改名
- [x] `app-tui-design-playground.feature` 新增 `@req:adp10 @human 场景: temporal-flow-tiling`（交互原型模块 MUST 以声明式时态图呈现不同时态并支持动态播放；MUST NOT 以键盘黑盒模拟为唯一呈现）
- [x] BDD 成对锁改：`tests/bdd/steps_app_tui_host.rs`、`tests/bdd/bindings_app_tui_transcript.rs`、`tests/tui_e2e/pty.rs` 等命中处
- [x] 验证：`cargo test --test bdd` 全绿；`llman sdd validate c2550-rename-terms-add-flow-view --strict`

## T5 交接卡遮盖修复 [blocked-by: T2]

- [x] `shell.css`/`index.html`/`main.ts`/`handoff.ts`：单行 sticky 条（复制按钮 + endpoint + 展开 ▾）+ 展开面板（非 sticky 流区块）+ 与 `#detail` 背景边框隔离；保留 `id="copy-handoff"`；≤960px 窄屏检查
- [x] 验证：`bun run --cwd designing/app check`

## T6 时态平铺图（3 个 tui-lab 模块）[blocked-by: T2]

- [x] `app/src/flow-view.ts` 渲染器 + `app/src/flow.ts` 类型（FlowDoc 解析校验）；`main.ts` 路由 `__flow__` chip「时态图」；删 `app/src/sim.ts` 与 live 机制、live CSS
- [x] 3 模块 `flow.yaml`（toast-stack / paste-fold / interrupt-arm）+ 补缺失 states 快照；删 3× `sim.ts`
- [x] `scripts/gen_designing_index.py` 标注 flow 有无；重生成 AGENT-INDEX
- [x] 验证：`bun run --cwd designing/app check` + `just qa`（check_tui_designing）

## T7 draft 黑话白话对照 + alignment 键 [blocked-by: T2]

- [x] alignment 键 `chrome`→`fixed`（全部 draft.yaml + `types.ts` + `main.ts`），UI dt 中文标签（固定区/对话条目/待办栏）
- [x] activity-fold（簇/信封）、loaded-resources（刷墙）、transcript（墙/轨）、errors（硬拒闸）等首现处加白话对照
- [x] 验证：`just gen-designing-index` + `just qa`

## T8 收口 [blocked-by: T3 T4 T5 T6 T7]

- [x] 零残留校验：`rg -n "壳层通告|尾随" ` 全仓仅剩归档区 + 词表弃用表；`rg -n "chrome" src/ designing/ docs/` 仅剩 packages fork 与词表映射行
- [x] `just qa` 全绿 → `just open-designing` 浏览器人工验收（3 时态图 + 右栏新布局 + toast-notice 路由）
