# c97-fix-archived-deferred-items Tasks

## Phase A — Markdown 渲染修复（已完成）

- [x] 链接样式添加 `link_styled` 配置项（默认 false），修复 spec r2 违规
- [x] 表格渲染添加 `Table/TableHead/TableRow/TableCell` 处理逻辑（`│` 分隔符 + 表头加粗）
- [x] 更新 `md_table` 快照测试
- [x] `cargo clippy` 0 warnings + 全部测试通过

## Phase B — Defer 清单盘点与分类

- [x] 审查 c92-refactor-code-hygiene 的 4 项 defer → 已分类（见下方）
- [x] 审查 c94-fix-race-conditions 的 5 项 defer → 已分类
- [x] 审查 c95-fix-test-stability 的 7 项 defer → 已分类
- [x] 审查 c96-refactor-architecture 的 6 项 defer → 已分类
- [x] 审查 c93-fix-resource-boundary 的 2 项 defer → 已分类
- [x] 为仍有价值的 defer 项创建独立 change proposals:
  - c98-refactor-tool-args-and-app-split（4 项：args helper、迁移、app.rs 拆分、错误类型）
  - c99-refactor-approval-and-bootstrap（6 项：ApprovalHub 重构、bootstrap 提取、model DTO）
  - c100-improve-test-harness（3 项：timeout helper、async timeout、MockToolContext）

### Defer 项分类结果

#### c92-refactor-code-hygiene（4 项）
| 原始 Defer 项 | 分类 | 理由 |
|---------------|------|------|
| 提取 `args.rs` helpers | **独立变更** | 7 个工具的参数解析可统一提取，有明确收益 |
| 7 工具参数解析迁移 | **独立变更** | 联动上项 |
| 拆分 `app.rs`（1600+ 行） | **独立变更** | 体量大，建议专项重构 |
| 统一 `pub(crate)` 可见性 | **取消** | 与 c96 联动，且收益低 |
| 统一错误类型约定 | **独立变更** | 架构决策，需专项讨论 |

#### c94-fix-race-conditions（5 项）
| 原始 Defer 项 | 分类 | 理由 |
|---------------|------|------|
| 重构 ApprovalHub（channel） | **独立变更** | 较大重构，需设计评审 |
| ApprovalHub Entry API | **独立变更** | 联动上项 |
| edit/write mtime 校验 | **取消** | atomic write 已解决核心问题，mtime 校验性价比低 |
| Session ensure 捕获 | **取消** | 需了解上游 adk-session API，目前无实际问题报告 |
| MCP 锁粒度优化 | **取消** | Arc clone 语义已足够，低优先级 |
| history 文件锁 | **取消** | 多实例场景低优先级 |

#### c95-fix-test-stability（7 项）
| 原始 Defer 项 | 分类 | 理由 |
|---------------|------|------|
| 配置 loader/secret 改用 TempDir | **取消** | 已有 ENV_LOCK 串行化，够用 |
| 创建 `with_test_timeout` helper | **独立变更** | 统一 test harness 改造有长期收益 |
| async 测试包裹 timeout | **独立变更** | 联动上项 |
| bash/hooks 缩短 sleep | **取消** | hook 测试已用 50ms，够快 |
| 外部命令测试标记 unix | **取消** | 低优先级 |
| 审查未使用 dev-deps | **取消** | 低优先级 |
| MockToolContext workspace_root | **独立变更** | 与工具重构联动 |
| paths fallback 断言 | **取消** | 低优先级 |

#### c96-refactor-architecture（6 项）
| 原始 Defer 项 | 分类 | 理由 |
|---------------|------|------|
| 抽象 model 解析 DTO | **独立变更** | 大型重构，需设计文档 |
| security wrap 移入 bootstrap | **独立变更** | 联动 bootstrap 提取 |
| 提取 bootstrap.rs | **独立变更** | 需全面理解三入口，专项工作 |
| Print/TUI/ACP 改用 bootstrap | **独立变更** | 联动上项 |
| 审查 default features | **取消** | 需评估下游影响，当前无紧迫性 |
| 添加 `full` feature alias | **取消** | 联动上项 |

#### c93-fix-resource-boundary（2 项）
| 原始 Defer 项 | 分类 | 理由 |
|---------------|------|------|
| zstd 解压大小限制 | **取消** | 低优先级，session 压缩数据来源可控 |
| ngram_set 容量上限 | **取消** | 低优先级，RepeatDetector 窗口本身有限 |

### 汇总

- **需要独立变更**：12 项（建议拆为 3 个独立 change proposals）
- **取消**：15 项（已无需求或性价比过低）

## Phase C — 流程改进

- [x] 在 AGENTS.md 中补充 defer 归档规范（defer 项必须关联后续 change 或明确取消）
