---
depends_on: []
title: 产品 TUI 预览：场景目录与假 Host，取代 /debug 斜杠
branch: sdd/c2930-add-tui-preview
base_branch: main
base_sha: 3f174a54060b7aabba8043695340974c853443aa
---

# 产品 TUI 预览：场景目录与假 Host，取代 /debug 斜杠

## Why

今日把「切到某一帧 UI 态」挂在产品斜杠 `/debug <scene>` 上，且仅 debug 构建注册。这不是目录，是藏在对话输入里的手测口：

- 产品 slash 词表被调试入口污染；DESIGN 已禁止常驻 debug 条，却仍把场景加载写成 slash。
- 场景注入与产品 TUI（attach 真 Host）共用同一组合根，心智是「在真会话里灌夹具」，而不是「打开预览组合」。
- 人与 agent 要对同一套真组件讨论 UI，需要 **场景目录 + 画布**（Storybook 形态），入口必须离开 slash。

需要的可观察行为：从 **交互表面的 designing 叶** 打开产品 TUI 画布；画布挂假 Host（磁带 / 注入，不跑用户仓库副作用）；用槽切场景；产品 `xylitol tui` 与 slash 不再出现 `/debug`。designing **不是** 顶层表面（与 `tui` / `print` 并列会挡住以后 `desktop` 等同构叶）。

## What Changes

- 新增 capability `app-tui-preview`：预览是 **TUI 内** 的组合（同一套 client 组件），不是网页第二套 UI，也不是产品默认入口。
- CLI：debug 构建提供 `xylitol tui designing`（`tui` 的叶，与 `run` 并列）。release 构建 **不得** 列出或接受该叶。不得提供顶层 `preview` / `designing`，不得提供 `tui --designing` 旗标。TTY 裸跑仍默认产品 TUI。预览 **不得** 适用产品 TUI 的 attach 未在听即失败。
- 预览组合打开时必须有 **场景槽**（固定区「槽」）：列出场景 id 与描述；选中后把该场景注入同一 TUI 画布。切场景用槽，不用 slash。
- 注入必须在本进程、隔离会话（不得覆写用户当前 session 文件）；未知场景须提示且不得崩溃。注入 API **不得** 出现在跨进程信封 / 远程 unary。
- 产品 slash **不得** 再解析 `/debug` 与 `/debug <scene>`（含空格与冒号形式）；SlashCommandSource 不得列出 debug。既有用 `/debug` 装夹具的验收场景改走预览注入。
- 既有场景表（多轮树、labeled、branched、live tape 等）迁成预览场景源，语义保持：假 Host 注入投影，不在预览里真 exec 用户仓库。

## Out of Scope

- 框选覆盖层 / `InspectFrame` JSON / 薄 HTML overlay / Inspect wasm（另 change；本提案只钉 TUI 内预览组合）。
- 用 xstate 解释器驱动产品 TUI。
- 把预览做成 TTY 裸跑默认入口，或做成 gpui / Web 产品面。
- 其它交互表面的 designing 叶（如未来 `xylitol desktop designing`）：本 change 只钉 `tui designing` 的形状，不实现第二套端。
- 预览进程内真 bash / 真 MCP / 真 Trust 项目门禁。
- 单包 Cargo `image` feature（与预览无关）。

## Capabilities

- `app-tui-preview`（新建）
- `cli-entry`（表面动词不含 preview；attach 门例外覆盖 tui designing）
- `app-tui-commands`（废止 `/debug`）
- `app-tui-session-tree`（夹具装载步骤改预览）

## Impact

- 用户：开发与设计协作改走 `xylitol tui designing`（仅 debug 构建）+ 场景槽；日常 `xylitol` / `xylitol tui` 不再有 `/debug`；release `--help` 看不见 designing。
- 测试：主机泵与 CLI 解析改绑 `tui designing`；session-tree 夹具场景改「预览注入」而非提交 `/debug …`。
- 文档：`DESIGN.md` 布局硬规则去掉「调试走 `/debug`」，改为预览组合才有场景槽；产品 TUI 仍无常驻 debug 条。

## Further Notes

权衡与切片见同目录 `design.md`。框选 / wasm 宿主不在本 change 实施。
