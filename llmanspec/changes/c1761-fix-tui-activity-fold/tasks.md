# Tasks: c1761-fix-tui-activity-fold

> 决策见 `proposal.md`；嵌套/配置/性能见 `design.md`；视觉 MUST 见 `src/app/tui/design/activity-fold.md`。
> **硬禁**：默认分支改 live specs；把 Compaction 头行 quick 混进本分支。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 Designed 规划壳 | 进行中 | proposal + design + tasks |
| 1 Specs landing | 未开始 | 须 Branch binding 后 |
| 2–7 Apply | 未开始 | `readyToImplement` 后 |

---

## 0. Designed

- [x] 0.1 `proposal.md`：嵌套、可见地板、`stream_collapse` 语义名、与 c1760/c2045 边界
- [x] 0.2 `design.md`：树、簇主刀=助手正文、YAML、鼠标/键盘、live window 刷新
- [x] 0.3 本 `tasks.md`
- [x] 0.4 `research/streaming-live-window.md`：流式 -1/-2/-3、Planning next moves、Ask
- [x] 0.5 视觉锁：`src/app/tui/design/activity-fold.md` + playground 槽 `activity-fold`（离散芯片，无滑入）

## 1. Specs landing

- [x] 1.1 干净树且在默认分支：`llman sdd change start c1761-fix-tui-activity-fold`（Compact quick 已单独提交后再 start）
- [x] 1.2 `app-tui-transcript`：改写 att23–att28、att31；**新增** att33 live window、att34 簇切刀
- [x] 1.3 `runtime-config`：`tui.activity_fold` 键、缺省、非法 `stream_collapse` 失败
- [x] 1.4 场景：对应 req 各一条 `feature: false` unit；禁止 toon `feature: true`
- [ ] 1.5 commit Specs landing → `readyToImplement=true`

## Apply backlog

### 2. 配置面

- [ ] 2.1 `TuiConfig.activity_fold` + 枚举 `envelope` / `clusters`；schema / example 模板
- [ ] 2.2 映射到 `ActivityFoldSettings`；host 启动装入；`enabled: false` 全细账
- [ ] 2.3 配置单测：缺省、解析、非法枚举失败

### 3. 嵌套状态

- [ ] 3.1 簇切分主刀 = 助手正文（thinking 不切）；live ≡ rebuild
- [ ] 3.2 信封展开态 + 每簇展开态；与 `ScrollbackFold` 分离
- [ ] 3.3 `toggle` 定点一级；`expandNearest` / `collapseNearest` 走嵌套栈

### 4. Paint + 持久头行

- [ ] 4.1 信封/簇展开后仍画头行 + `▾` 并登记 hit
- [ ] 4.2 信封折叠只画 Worked for；不登记内层 hit
- [ ] 4.3 User 外显；信封折叠时只留 **最后** Assistant；Todo/Compaction **进**信封；ScrollNotice/Error 不进
- [ ] 4.4 流式当前 turn：live window（不套信封）；-1 尾行状态机；-2 进行时簇头
- [ ] 4.6 打开簇 = sealed 队列 + inflight 尾行；点 Planning = 展开 -2；三角只在 -2 且 sealed 非空
- [ ] 5.4 展开只画已缓存摘要 + entry 下标；禁止点击时全量重分区

### 5. 增量刷新

- [ ] 5.1 ToolEnd 只更新 -2；ThinkingDelta 只更新 -1；-3 冻
- [ ] 5.2 TextDelta MUST NOT 重算 -2/-3 / 全历史 MD
- [ ] 5.3 toggle 遵守 ath25

### 6. 验证

- [ ] 6.1 harness：展开后再点头行可折；嵌套两级鼠标互不误伤
- [ ] 6.2 resume/`auto_on_rebuild`：超窗 `Worked for`；近窗信封展开
- [ ] 6.3 流式：无正文见 Planning next moves；ToolEnd 后 -2 变、-3 不变；Ask Waiting 可点
- [ ] 6.4 `just fmt` + 相关测

### 7. 收口

- [ ] 7.1 `llman sdd validate c1761-fix-tui-activity-fold --strict --no-check`
- [ ] 7.2 确认未把 Compact 头行 / L1 四类重做算进本票

## 实现顺序

```text
0 Designed
  → 1 Specs landing（干净树 start）
  → 2 YAML → 3 状态/簇 → 4 paint → 5 增量 → 6 harness → 7 validate
```

## 测试边界（seam）

| seam | 覆盖 |
|---|---|
| `HostSession` + 合成 Mouse / 键 | 头行往返、嵌套定点、nearest |
| `render_scrollback` / `ActivityFoldState` | 信封/簇可见性、hit 登记 |
| `AppConfig` YAML | `tui.activity_fold` 缺省与非法值 |
| paint cache misses | ath25 |

无新 CLI 子命令。`.feature` 可执行场景本票默认不新增（unit / harness）。
