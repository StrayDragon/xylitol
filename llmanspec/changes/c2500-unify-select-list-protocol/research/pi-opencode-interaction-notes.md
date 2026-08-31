# pi / opencode 交互机制调研（c2500 深挖沉淀，2026-09-01）

> 结论：xylitol 现状（resume 自绘 panel + 各槽 SelectList）与上游 pi 同构，行为一致好用；
> opencode 的「全局命令化」方向有价值但与本仓架构不合，其机制沉淀于此供 c2505（命令面板）
> 与未来「对当前选中项的动作」类需求参考。

## ../pi（xylitol-tui 上游）

- **编辑器槽替换**（`coding-agent/src/modes/interactive/interactive-mode.ts` `showExtensionCustom`）：
  非 overlay 模式下 `editorContainer.clear()` 移除编辑器 → selector 入槽 → `setFocus(component)`；
  关闭时 `restoreEditor()` 恢复编辑器与已存文本。**= 选择器打开时编辑器不渲染**（用户直觉与此一致）。
  overlay 模式仅用于悬浮场景（`ui.showOverlay`）。
- **session-selector**（1031 行，自绘）：槽内 search 输入 + 键位反查提示
  （`keyHint("app.session.delete", "delete")`）+ `app.session.rename` / `app.session.delete`
  经 KeybindingsManager 匹配 + delete 确认态拦截。**= xylitol resume panel 的原型**。
- SelectList 组件接口与 xylitol 完全同源（value/label/description + setFilter + 选中）。

## ../opencode（packages/opencode/src/cli/cmd/tui/，opentui/solid）

- **命令面板 = 通用列表的另一视图**（`component/command-palette.tsx`）：条目全部来自
  keymap 注册表反查（title/desc/键位提示/`suggested` 分组），零手工维护——与 c2505
  提案「keybindings manager 反查、永不失同步」一致。
- **次级动作 = dialog 上下文命令**（`ui/dialog-select.tsx` 的 `actions` 槽）：如 session-list
  声明 `command: "session.rename"`，键位经 `getCommandBindings` 反查渲染；dialog 打开期间生效。
  **没有 action 条、没有 Tab 焦点环**——打印字符归过滤输入，动作键是 keymap 注册的组合键。
- rename 为独立 dialog（`dialog-session-rename.tsx`），操作「当前选中项」上下文；
  `Dialog` 基座 = 居中模态 + 暗背景 + dialog stack。
- **对本仓不合点**：全局 overlay（暗背景 + z-index 覆盖 + 下方持续渲染）与 xylitol 的
  slot 槽替换模型差异大，实现成本高；且上下文命令机制依赖 keymap 的 dialog-scoped 注册。

## 对 c2500 的裁定

行为保留、UI 变更出局、resume 不动；原提案的分组头 / details / action 条 / Tab 焦点 /
上下文命令全部不做。过滤语义差异（models 的 `fuzzy_filter` vs `SelectList::set_filter`
前缀匹配）保留并参数化，不统一。

## 对未来的指针

- c2505 命令面板：opencode 的 palette 实现是最佳参照（注册表反查 + suggested 分组）；
- 「对当前选中项的动作」类需求：opencode 的 dialog-context commands 机制值得再评估
  （前提是 keybindings manager 增加 dialog-scoped 注册）；
- 分组头（suggested 分组）若将来需要，opencode 的 category 分组渲染可直接参照。
