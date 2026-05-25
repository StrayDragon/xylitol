# Feature Request: llman SDD — Defer 追踪与归档完整性检查

> 来源项目：xylitol
> 日期：2026-05-25
> 优先级：High
> 状态：Draft

## 背景与问题

在 xylitol 项目使用 llman SDD 工作流的实践中，发现了一个系统性的流程缺陷：

**变更可以在存在大量未完成（defer）tasks 的情况下通过 `llman sdd validate --strict` 并被正常归档。**

### 实际数据

对 2026-05-24 批次归档的 7 个变更进行审计，发现：

| 变更 | 总 tasks | 已完成 | 已 defer | 完成率 |
|------|---------|--------|----------|--------|
| c91-fix-security-enforcement | 11 | 11 | 0 | 100% |
| c93-fix-resource-boundary | 10 | 8 | 2 | 80% |
| c90-update-markdown-rendering | 7 | 4 | 3 | 57% |
| c96-refactor-architecture | 11 | 5 | 6 | 45% |
| c94-fix-race-conditions | 10 | 4 | 5 | 40% |
| c92-refactor-code-hygiene | 10 | 4 | 4 | 40% |
| c95-fix-test-stability | 11 | 3 | 7 | 27% |

**汇总**：70 个 tasks 中 27 个（39%）被标注 defer 后直接归档，没有任何一个 defer 项产生了后续 change proposal。

### 实际影响

- **Spec 违规**：c90 的 spec r2 要求"链接样式不应强制"，但代码实现硬编码了 LightBlue+Underline。由于 defer tasks 未被追踪，此违规在归档后未被发现。
- **技术债积累**：27 个 defer 项散落在归档目录中，没有集中的可见性，极易被遗忘。
- **工作流信任度下降**：`validate --strict` 通过 + 已归档 ≠ 已完成，这削弱了 SDD 工作流的可信度。

## 功能需求

### FR-1: tasks.md 完成度检查（validate 增强）

`llman sdd validate --strict` 应检查 tasks.md 中未勾选（`[ ]`）的 tasks：

- **选项 A（严格模式）**：所有 `[ ]` 项视为验证失败，除非标注了 `(defer → <target-change-id>)` 链接。
- **选项 B（警告模式）**：`[ ]` 项产生 warning，`--strict` 下 warning 升级为 error。

建议默认使用 **选项 A**：defer 必须显式链接到后续变更。

```yaml
# config.yaml 中可配置的策略
rules:
  tasks:
    - "Break tasks into chunks of max 2 hours."
    - "Unchecked tasks at archive time must link to a follow-up change via `(defer → <change-id>)` syntax."
  archive:
    - "All tasks must be checked or have a valid defer link before archiving."
    - "Minimum task completion ratio for archiving: 50%"  # 可配置阈值
```

### FR-2: 结构化 defer 语法

引入标准化的 defer 标注语法，让工具可以解析和追踪：

```markdown
# 当前（非结构化，无法追踪）
- [ ] 重构 ApprovalHub（defer - 需要较大重构）

# 建议（结构化，可追踪）
- [ ] 重构 ApprovalHub (defer → c98-refactor-approval-hub)
- [ ] 清理未使用 dev-deps (cancelled — 已无需求)
```

工具行为：
- `defer → <change-id>`：验证 target change 存在于 `llmanspec/changes/` 中
- `cancelled`：标记为已取消，不阻塞归档
- 无标注的 `[ ]`：阻塞归档（在 `--strict` 下）

### FR-3: 孤儿 defer 检测命令

新增 `llman sdd orphans` 命令，扫描所有已归档变更的 tasks.md，列出未被后续变更追踪的 defer 项：

```bash
$ llman sdd orphans

Found 27 orphaned defer items across 6 archived changes:

  c92-refactor-code-hygiene:
    - [ ] 提取 src/agent/tools/args.rs helpers (defer)
    - [ ] 将 7 个工具的参数解析迁移到新 helper (defer)
    ...

  c95-fix-test-stability:
    - [ ] 配置 loader/secret 测试改用 tempfile::TempDir (defer)
    ...

Suggestion: Create follow-up changes or cancel these items.
```

### FR-4: 归档前 completion ratio 门控

在 `llman sdd archive` 命令中加入完成度门控：

```bash
$ llman sdd archive c95-fix-test-stability

⚠ Task completion: 3/11 (27%)
⚠ 7 unchecked tasks without defer links:
  - [ ] 配置 loader/secret 测试改用 tempfile::TempDir
  ...

Archive blocked. Options:
  1. Add defer links (defer → <target-change-id>) to pending tasks
  2. Cancel tasks that are no longer needed (cancelled)
  3. Use --force to archive anyway (not recommended)
  4. Complete the remaining tasks
```

### FR-5: DAG 图中显示 defer 关系

`llman sdd graph` 生成的依赖关系图应包含 defer 链接作为虚线边：

```
c90-update-markdown-rendering ···defer···> c97-fix-archived-deferred-items
c94-fix-race-conditions ···defer···> c98-refactor-approval-hub
```

这使得 defer 的"接力关系"在项目全局可见。

## 实现建议

### 优先级排序

1. **P0 — FR-1（validate 增强）**：最小改动，最大收益。在现有 validate 逻辑中增加 tasks.md 解析即可。
2. **P0 — FR-2（defer 语法）**：FR-1 的基础。先定义语法标准，其他功能依赖此语法。
3. **P1 — FR-4（archive 门控）**：阻止问题再次发生的核心守卫。
4. **P2 — FR-3（orphans 检测）**：存量清理工具，一次性扫描。
5. **P2 — FR-5（DAG 显示）**：可视化增强，锦上添花。

### 向后兼容

- 现有不带 defer 链接的 `[ ]` 项应在升级后产生 warning（而非 error），给项目迁移窗口。
- 引入 `config.yaml` 中的 `rules.archive.strict_defer: false` 配置项作为过渡开关。
- 旧的 `(defer - reason)` 文本格式仍然被解析为"无链接 defer"，产生 warning 提示用户迁移。

## 参考

- 来源审计：xylitol 项目 `llmanspec/changes/archive/2026-05-24-*` 批次归档
- 相关修复：xylitol `c97-fix-archived-deferred-items` 变更提案
