---
rules_edit_acked: true
depends_on: []
branch: sdd/c2550-rename-terms-add-flow-view
base_sha: 267e8fbfd94be05210cd17d7eec74c4e3cc8da78
checkpointed: true
checkpoint_sha: 267e8fbfd94be05210cd17d7eec74c4e3cc8da78
---

# 设计稿术语定型与 playground 体验改造

## Why

「壳层通告」「尾随」「chrome」三个讨论词对人类读者困惑：壳层易误读为 OS shell、尾随不像插入语义、chrome 是外来行话且在代码中身兼 toast 专指与泛固定区两种语义。同时 designing playground 有三个体验问题：右栏 handoff 悬浮块遮盖设计描述；对话框式键盘黑盒模拟不可平铺总览交互时态；draft 黑话（簇/信封/刷墙）首现无对照。术语与体验一起定型，一次改全、不留残留。

## What Changes

- 词表 SSOT（`docs/architecture/TUI信息面与固定区词汇.md`，文件同步改名 `TUI信息面与固定区词汇.md`）：
  - 壳层通告 → **通知条**（toast notice · `ToastNotice`；英文锚与代码标识符同步换，`chrome toast` 弃用）
  - 尾随 → **尾插**（tail-append；顶插不变）
  - chrome → **固定区**（fixed zone；状态条/页脚/通知条/槽等非滚动固定框架区统称）
  - 弃用表新增三行（壳层通告→通知条、尾随→尾插、chrome→固定区）；归档 change 保留旧词作史实
- 代码标识符（产品面 `src/`，`packages/xylitol-tui` fork 内泛指用法不改）：
  - toast 专指：`push_chrome_toast`→`push_toast_notice`、`chrome_toast*`→`toast_notice*`、`render_chrome_toast_slot`→`render_toast_notice_slot` 等
  - 泛固定区：`ChromeOp`→`FixedZoneOp`、`PreviewInject::Chrome`→`::FixedZone`、`SlotChromeRows`→`SlotFixedZoneRows`、`chrome_footprint*`→`fixed_zone_footprint*`、`sync_runtime_chrome`→`sync_fixed_zone`、`set_active_chrome`→`set_active_fixed_zone`、`refresh_chrome_caches`→`refresh_fixed_zone_caches`、`effects/slash/chrome.rs` 改名
  - designing 模块 id `chrome-toast` → `toast-notice`（目录/路由/引用同步）
- capability 改名：`app-tui-chrome` → `app-tui-fixed-zone`（含场景 id chrome-* → fixed-zone-* / toast-notice-*）；其余 feature 中文语句与场景 id 词同步
- designing playground：
  - handoff 悬浮块改单行条（复制按钮 + endpoint + 展开面板），不再遮盖右侧描述；组件名不改
  - 新增声明式 `flow.yaml` 时态平铺图（节点=states 快照、边=按键/超时转迁、demo 播放路径），仅先落地 3 个 tui-lab 交互原型模块（toast-stack / paste-fold / interrupt-arm），删除键盘黑盒 sim 机制
  - alignment 键 `chrome` → `fixed`，UI 显示中文标签（固定区/对话条目/待办栏）
  - draft 黑话（簇/信封/刷墙/词形等）首现处加白话对照

## Capabilities

- `app-tui-fixed-zone`（改名自 `app-tui-chrome`）：词同步 + 场景 id 同步，行为语义不变
- `app-tui-input` / `app-tui-commands` / `app-tui-host` / `app-tui-session-tree` / `app-tui-transcript` / `agent-todo` / `server-core` / `app-tui-bridge` / `package-tui-theme` / `package-tui-agent-demo` / `package-ai-bridge` / `app-tui`：纯措辞词同步（壳层通告→通知条、chrome→固定区、尾随→尾插），MUST 语义不变
- `app-tui-design-playground`：新增 @req（交互原型模块 MUST 以声明式时态图呈现不同时态并支持动态播放，MUST NOT 以键盘黑盒模拟为唯一呈现）

## Impact

- wire 协议形状不变（改名均在 host 内部标识符与中文文案层；`ChromeOp` 为 debug fixture 本地枚举，不经 wire 序列化）
- BDD step 注册文案与 feature GWT 文案成对锁改，`just qa` 抓失配
- designing 数据键 `alignment.chrome` 改名波及全部 draft.yaml（机械替换）+ `types.ts`/`main.ts`
- 指针层文件名引用（根 AGENTS、docs/AGENTS、README、术语表、src/packages/designing AGENTS、roadmaps）随词表文件改名更新
