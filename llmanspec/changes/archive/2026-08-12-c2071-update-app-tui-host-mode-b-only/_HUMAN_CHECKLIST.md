# c2071 产品 TUI 人验清单（ApplicationOwned 默认）

> **对象**：产品 `xylitol` TUI（不是 `agent_demo`）。
> **分支**：`sdd/c2071-update-app-tui-host-mode-b-only`
> **自动化对照**：`[_VERIFY.md](./_VERIFY.md)` · `[_SMOKE_PTY.md](./_SMOKE_PTY.md)`
> **签字列**：Ghostty / 本机终端 · tmux · SSH（远端）——每格填 `PASS` / `FAIL` / `SKIP` + 日期。

## 0. 启动（各环境各跑一次）

```bash
eval "$(just cargo-wt-env)"

cargo run
# 或：已有配置的真模型
```


| 环境                  | 进 alt（屏「换场」） | 无 `XYLITOL_TUI_MOUSE` 仍可拖选 | `/exit` 后主屏可上翻会话 | 签字       |
| ------------------- | ------------ | -------------------------- | ---------------- | -------- |
| Ghostty / 本机        | pass         | pass                       | pass             | pass     |
| tmux（本机）            | pass         | pass                       | pass             | pass     |
| SSH → 远端 tmux/裸 TTY | not test     | not test                   | not test         | not test |


**PASS 口径**：进入后是 alt-buffer（不是主屏差分贴底）；鼠标拖选有应用高亮；退出后主屏 scrollback 里仍能看到本轮对话（dump）。

---



## 1. AO 机制（ath30 / ath29 / ath31）


| #   | 项           | 怎么看                                                                 | G    | tmux | SSH      |
| --- | ----------- | ------------------------------------------------------------------- | ---- | ---- | -------- |
| A1  | 默认即 AO      | 启动即 alt；**不要**设 `XYLITOL_TUI_MOUSE`                                 | pass | pass | not test |
| A2  | dock 排除     | 拖选 transcript 进 editor/footer：**夹底续选**，不选中输入面文字                     | pass | pass | not test |
| A3  | Editor 独立选  | 在输入框多行拖选：只选草稿，不污染 transcript 选区                                     | pass | pass | not test |
| A4  | 松手复制        | 非空选松手 → 剪贴板有内容（OSC52/本机）                                            | pass | pass | not test |
| A5  | Copied 提示   | 复制成功 → chrome 短「Copied」类提示（**非** `Error:` toast、**非** ScrollNotice） | pass | pass | not test |
| A6  | Moved 不闪    | 空闲时鼠标空移：无明显整屏闪烁/重绘感                                                 | pass | pass | not test |
| A7  | teardown 干净 | `/exit` 后：鼠标模式关、回到主屏、无残破 CSI/光标乱跳                                   | pass | pass | not test |
| A8  | 无模式开关       | 设置/slash **找不到** Inline↔AO 切换；无热切手感                                 | pass | pass | not test |


---



## 2. 产品业务面（应与旧 Inline 默认一致）

> 设计基线：Driver / slash / 键位 / chrome **语义不变**；只允许 AO 视口/选区/鼠标/dump 差异。


| #   | 项                 | 操作要点                                        | G    | tmux | SSH |
| --- | ----------------- | ------------------------------------------- | ---- | ---- | --- |
| B1  | 提交一轮              | 打字 → Enter → Fake/模型流式出现                    | pass |      |     |
| B2  | abort             | 一轮中 Ctrl+C / Esc：停流，可再提交                    | pass |      |     |
| B3  | steer / follow-up | busy 时再输入：queue strip / 下轮预告行为正常            | pass |      |     |
| B4  | `/model`          | 打开选模槽；左右 thinking；确认后页脚/cue                 | pass |      |     |
| B5  | `/exit`           | quit + dump（见 A7）                           | pass |      |     |
| B6  | bang `!`          | 若本机有 shell：`!echo hi` 或短命令；Esc 可取消挂起        | pass |      |     |
| B7  | overlay           | `/mcp` 或模型槽：Esc 先关槽，不误退出                    | pass |      |     |
| B8  | resize            | 缩/放终端：布局不崩；过小有 hint、恢复后可用                   | pass |      |     |
| B9  | 长 transcript      | 多轮或粘贴长文：滚轮上翻粘滞/跟底合理；底栏 dock 仍可用             | pass |      |     |
| B10 | suspend 编辑        | Ctrl+G（或产品外编）进出：resume 后仍在 AO（alt+mouse 仍在） | pass |      |     |
| B11 | 主题/设置             | `/theme` 等即时设置：页脚/cue 更新，无假 ScrollNotice 墙  | pass |      |     |
| B12 | reload（可选）        | `/reload`：Reloading 态 + 拒闸文案；取消路径若关心再验      | pass |      |     |
|     |                   |                                             |      |      |     |


---



## 3. 性能 / 手感指标（你关心的主观+可记数）

> 无专用 perf harness 时用「体感 + 简单墙钟」。任一项持续恶化 → 记 WARNING，不一定挡 archive。


| #   | 指标             | 怎么测                  | 目标感（软）                                    | 实测 / 备注 |
| --- | -------------- | -------------------- | ----------------------------------------- | ------- |
| P1  | **空闲 CPU**     | top/`pidstat`：静止 10s | 接近 0（偶发 tick 可）                           |         |
| P2  | **空移鼠标**       | 在 transcript 上空移     | 无明显重绘闪烁；CPU 不飙                            |         |
| P3  | **拖选帧感**       | 快拖选大块                | 高亮跟手；卡顿 < 偶发                              |         |
| P4  | **流式刷屏**       | Fake/模型长流            | 跟底流畅；输入区仍可打字                              |         |
| P5  | **长史滚动**       | ≥50 轮或大 paste 后滚轮    | 不「一顿一帧」；跟底切换不抖                            |         |
| P6  | **首帧进 AO**     | 启动到可输入               | 无长空白；无二次闪进闪出                              |         |
| P7  | **退出 dump 体积** | 长会话 `/exit` 后主屏上翻    | dump 完整可读；终端不假死                           |         |
| P8  | **resize 成本**  | 连续拖拽窗口边缘             | 不掉帧成幻灯；layout 稳定                          |         |
| P9  | **SSH 延迟**     | 远端高 RTT              | 键入/拖选仍可用；复制不丢                             |         |
| P10 | **tmux 嵌套**    | tmux 内再开             | mouse 不与 tmux 抢到不可用（可记需 `set -g mouse` 否） |         |


可选量化（有兴趣再记）：

```bash
# 空闲采样示例（另开窗）
pidstat -p $(pgrep -n xylitol) 1 10
```

---



## 4. 负向 / 回归哨兵


| #   | 不应出现                                  | 看到则 FAIL                    |
| --- | ------------------------------------- | --------------------------- |
| N1  | 产品默认主屏 Inline（无 alt）                  | 启动像旧差分贴底、终端原生选区才是唯一手段       |
| N2  | 依赖 `XYLITOL_TUI_MOUSE=1` 才有选区         | env 未设却完全不能拖选               |
| N3  | 退出后 mouse/alt 残留                      | 壳层仍吃鼠标、屏错乱                  |
| N4  | Copied 写成 Error toast / 滚进 transcript | ath31 违例                    |
| N5  | 业务键位漂移                                | Ctrl+C / Enter / slash 语义变了 |


---



## 5. 签字汇总


| 环境           | 日期  | 签字人 | 总评  | 阻塞项 (#) |
| ------------ | --- | --- | --- | ------- |
| Ghostty / 本机 |     |     |     |         |
| tmux         |     |     |     |         |
| SSH          |     |     |     |         |


**合并建议**：A 段 + B1–B5 + P1/P2/P6 全 PASS 即可合；B6–B12 / P3–P10 可记 FOLLOW-UP。
