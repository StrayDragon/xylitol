# 设计：全仓 @req 前缀机械迁移为 r<数字>

## 约束

- 零行为变更：MUST/SHALL statement、场景名、验收步骤、标签分类一律不动。
- 全局唯一：新 `r<数字>` 不得与既有 `r\d+`（最大 76）或彼此冲突——
  validate 在结构全绿后启用全局查重 gate（`duplicatesFor`）。
- 引用保真：同一 capability 内同一旧 id（规则定义 + 验收场景 `@req:` 引用）
  必须映射到同一新号。
- 可审计：新旧映射落盘 `research/req-id-migration-map.md`，随 change 归档。
- 不手动改 64 个文件：用确定性的机械脚本生成 + 二次校验，不许手写。

## 分配策略

| 输入 | 规则 |
|---|---|
| 已合规 `@req:r<num>` | 原样保留（同号不变） |
| 非 r 前缀旧 id | 全局计数器从 1000 起，按文件语法序 + 文件内出现序逐个分配唯一 `r<N>` |
| 文件内重复引用 | 同一旧 id → 同一新号（用 per-file 映射表） |
| 全仓查重 | 新号落在 `r1000..r1999` 区间，与既有 `r1..r76` 无交集，天然唯一 |

计数器落在 `1000+` 使既有 `r<数字>` 与新增不打架；区间上限留足余量
（902 个旧 id < 1000 空位，单 change 够用）。

## 迁移流程（机械）

1. 扫描 `llmanspec/specs/**/*.feature`，收集每个文件内的 `@req:<id>` 集合。
2. 按文件语法序遍历；对每个非 r 前缀 id 分配 `r<1000+counter++>`，构造
   per-file `旧id → 新号` 映射。
3. 逐文件正则替换 tag（仅替换 `@req:旧id` 形式，不碰正文引用）。
4. 生成 `research/req-id-migration-map.md`（按 capability 分组表格）。
5. 更新 `src/app/mod.rs` 注释 `@req:tt08` → 新号。
6. 校验：
   - `llman-sdd validate --specs --no-check` 全绿；
   - 全仓唯一性断言（脚本二次核对新旧映射 1:1 且全局唯一）；
   - `grep` 确认无残留非 r 前缀 `@req:`；
   - BDD 场景名未变（bindings 走 `path+name`，无需改动）。
7. `git diff --stat` 人工审阅仅 tag 行变化。

## 回滚

单 commit 机械替换，`git revert` 即可整体回退；映射表保证可追溯。
