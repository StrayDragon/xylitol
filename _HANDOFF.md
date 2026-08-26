# _HANDOFF — 2026-08-27 interaction-injection 基建批

> 临时交接笔记（勿升格规范）。

## 本批完成（3 commits：445d7ab7 / ffe8917b / d9f73f79）

**interaction-injection 最小基建交付**（产品方法表窄门方案），BDD 348 → **363 全绿**：

1. 新管线 **E′ 无头键鼠交互挂载面** `InteractionBdd`（`src/app/tui/interaction_scene.rs`，
   正常编译路径仿 SceneBuilder tt08 先例）：
   - 键注入 = pre-focus listener 序（app.clear/app.interrupt 先行，同
     `install_ui_root_key_listeners`）→ focus/slot 路由；
   - 鼠标注入 = `UiRoot::click_fold_at(col,row)`（从 host hit_priority 闭包提取，
     host 与 BDD **同一管道**）；点击后须重绘再查 regions（真实引擎按新帧映射坐标）。
2. transcript 折叠族 9 条全转 executable：att7/20/21/22/25/29/30/31/32。
   关键几何事实：簇头三角 col_start=0；L1 块三角 col_start=2（rail 缩进）；
   Thinking id 形如 `{hash}-0`；收纳态不渲染内层也不登记内层三角（att25 即证）。
3. input 会话树键序族 6 条转 executable：ati22/24/25/26/27/36
   （FilterMode 循环/foldkeys/Shift+F fork pending/Shift+L 标签编辑 Esc 取消/
   Shift+Tab 负向）。和弦参数词「在树槽按下和弦 "Ctrl+U"」经 parse_chord 解析。
4. 产品修复一并入库：TreeSlot 标签编辑 Esc 取消语义原在 slot_nav::on_escape，
   BDD 注入路径此前绕过——修注入口而非改产品（SSOT 次序先怀疑自己）。

## 下次开工候选（按性价比）

| 方向 | 规模 | 说明 |
|---|---|---|
| **HostSession+ScriptedDriver 门** | ~12 条 ati busy 族 | ati2/3/10/14/16/19/20/28/30/31/32/43 需要 host 私有链导出（mod host + effects cfg(test) 泵升正常路径）。tests.rs 已有 TestTerminal/harness_* 单测可平移 |
| transcript 余 14 条 CONVERT | att1/4/8/9/11/12/14/15/16/18/23/26/27/28 | 与交互门无关：rail 渲染(管线 C/D)、bang 块、history rebuild(E seam)、auto-collapse 配置 |
| 三 candidate 新站照旧 | bridge 14 / otel 15 / tree-selector 14 | 管线 ABC 复用 |

踩坑新增：
- `edit` 工具大面积 replace 时容易误删相邻方法签名（本批伤过 tree_filter_for_test /
  tree_search_query_for_test 并已复原）——长文编辑前先精确 sed -n 拍原文。
- SceneBuilder.tool_end 的 result 硬编码 "ok"；多行 output 用 InteractionBdd.push_xy
  直发原始 XyEvent::ToolExecutionEnd。
- FilterMode 等 layout 层 re-export 带 `#[cfg(test)]` 时对集成测试不可见，转正需评审活消费者。

# _HANDOFF — 2026-08-23 会话交接

> 临时交接笔记（勿升格规范）。工作树干净，main 领先 origin/main 95 个提交，
> 本次 push 后归零。所有新工作均已提交。

## 今日完成

1. **外部 TUI/server 调研**（会话内两份深挖报告，未入库；关键结论已蒸馏进各票 research）
2. **SDD 草案六票**（server/tui 健壮性，`llmanspec/changes/c246*`）：
   幂等命令准入 / serve 就绪窗口 / client-local 存储通道 / 注册文件发现 /
   attach 重连状态机 / 凭据门禁（c2485 为决策草案，depends_on c2475）
3. **designing tui-lab 实验区**：`designing/tui-lab/modules/`（位置即语义，
   正文不写「未交付」）；playground 放行 `/tui-lab/<id>/<state>`；
   新增 **sim.ts 可交互原型层**——模块带 sim 即默认进入键盘驱动原型
   （已交付 paste-fold / toast-stack / interrupt-arm 三个，均实测通过）
4. **性能基线草案 c2490**（用户拍板：**当前最高优先级方向**）
5. **交互组件五票调研**（c2500–c2520，纯调研无代码）：SelectList 协议收敛 /
   命令面板 / 探索分组 / 键位速查层 / 宽度断点档位

## 下一步规划（建议顺序）

1. **c2490 建基线尺子**（最高优先级）：lab 形态测帧耗时分布 + 内存画像；
   外部渲染手法深挖 agent 曾被取消，推进时重跑一次并把手法写进其 research/
2. **c2500 SelectList 协议收敛** propose（交互票地基，其余面板类直接复用）
3. 其余票按依赖与拍板进度逐个 propose（全部停在 draft，未 attach 分支）

## 等待人类拍板的决策点

| 票 | 问题 |
|---|---|
| c2485 | 凭据门禁：默认生成 vs 配置开启 vs 维持放行；WS 兜底形态；「凭据不进工具子进程 env」是否先行 |
| c2505 | 入口键位：Ctrl+P（触碰 Plate 冻结决议）/ `/` 空触发即弹 / leader |
| c2515 | 形态：b) 即时速查面板先行 vs b+a) leader 前缀二期 |
| tui-lab interrupt-arm | 单 Esc 直接中止（现行）vs 双 Esc 二次确认（若采纳须改面 AGENTS 规则+测试） |
| tui-lab paste-fold | 整块单元可行方案：行内区间 vs 占位符入缓冲（编辑器现为 `Vec<String>` 纯文本行） |

## 工作约定备忘

- **去品牌约定**：灵感可提 opencode/codex 于对话，仓库工件一律中性化
  （research 用「外部参考实现」，proposal 问题导向）
- **designing 流程**：UI/UX 候选先入 `tui-lab`（可带 sim.ts 交互原型），
  拍板后 `git mv` 晋级 `tui/modules/` + surface 改回 + 补 harness/BDD
- 提交卫生：单 commit 收尾用 `finalize`；SDD 礼仪 `chore(sdd)`；
  designing 用 `docs(designing)`（hook 类型表无 design）
- ⚠️ `pkill -f <pattern>` 会匹配到自己的 shell 命令行（本会话踩坑两次）：
  杀 vite 请用精确进程名或先记录 PID
