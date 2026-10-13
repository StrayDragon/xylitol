# Design：瞬态会话详情抽屉（c2875 收敛形态）

> proposal 两问的落定：不做占用列（不动「无双栏」常驻分栏格局）；仅做**右缘瞬态会话详情抽屉**（overlay），依赖 c2858 已落地的指针管线（本期不注册命中，见下）。

## 决策

- **形态**：右缘 overlay 抽屉（≤30 列，transcript 区顶部向下），瞬态非驻留——`alt+i`（`app.session.drawer`，进键位表可重映射；alt+d 与包级 editor deleteWordForward 冲突已避让）显隐、复按或 Esc（`app.interrupt`）收起；任意槽位可切换（根输入头、先于槽派发）。DESIGN 硬规则 2 增例外注记（瞬态 overlay ≠ 双栏/常驻侧栏）。
- **内容**：会话身份（host drain 时供给）+ 根既有 footer 态（模型 / thinking / cwd）；只读快照，不自动刷新（重开即新）。
- **z 序**：抽屉先画、通知栈后叠——通知栈右上短暂遮盖抽屉，永不被抽屉抢盖。
- **指针**：本期不注册命中（纯展示）；AO 选区可覆盖抽屉文本为已知限制，后续票再议 `HitSpace` 扩展。
- **词表**：组件名「抽屉」（drawer）；不冒充待办栏 / 槽 / status。
- **落点**：根 `overlay_viewport` 渲染钩子（与 client notices 同族后处理）；`PendingSlotOps.session_drawer` 走既有 host drain 握手；无新依赖、无新 crate。

## 非目标

占用列、宽度动画、阴影/卡片 backdrop、命中注册与点按关闭、抽屉内交互、gpui 第二面。

## 深挖审计记录（2026-10-13）

逐点复核结论（实现已按此修正）：

1. **键位冲突（实伤，已修）**：首选 `alt+d` 与包级 editor `tui.editor.deleteWordForward`（alt+d / alt+delete）冲突，会在编辑面遮蔽词删除；改 `alt+i`（info 语义，全表无占位）。
2. **Esc 语义分级（实伤，已修）**：抽屉开启时 Esc 无条件收起会吞掉 busy 中止——改为忙碌态 Esc 透传中止链（词表 E 类：busy=中止优先），仅闲置态 Esc 收抽屉；收起仍可复按 `alt+i`。
3. **双 Esc 会话树**：抽屉收起路径不进 `last_esc_at` 双击链，无「收抽屉误开树」回归。
4. **槽内 Esc 取消握手**：抽屉开启期间槽级 Esc（取消 picker 确认）被后置——抽屉为最上层瞬态面，先收抽屉再操作槽，可接受。
5. **通知栈 z 序**：抽屉先画、通知后叠，永不被抢（render 顺序保证）。
6. **AO 选区穿透**：本期不注册命中，选区可覆盖抽屉文本——已知限制，后续票再议 `HitSpace` 扩展；不阻塞本票（c2858 管线已在，扩展纯增量）。
7. **窄宽**：box 宽 `min(30, width-2)`，极窄档退化为窄条截断，不 panic 不挤爆（TooSmall 由 host 另行处理）。
8. **缓存**：overlay 在帧后处理（cache 之外），显隐无需击穿 upper cache。
