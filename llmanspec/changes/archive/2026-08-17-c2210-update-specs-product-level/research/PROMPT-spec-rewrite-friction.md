# 派工 prompt：按审计表改写最高摩擦 specs（c2210 执行切片）

把下面整段交给执行 agent。只改审计表「最高摩擦 10 条」覆盖的 capability；**不要**一次重写全部 65 个 cap。

---

## 难度

中。概念简单（产品级 WHAT，不钉路径/类型/行数），但 toon/feature 文件多，**极易改歪语义**。每条对照 `audit-table.md` 的「改写草稿」；吃不准就 `keep` 并在 PR 里标出，不要猜。

## 在哪开工（硬）

**必须独立分支，禁止在默认分支改 `llmanspec/specs/**`。** 强烈建议 worktree（主仓同时可能改 TUI/AGENTS）：

```bash
wt switch --create --no-cd -y c2210-spec-rewrite
cd ../xylitol.c2210-spec-rewrite
eval "$(just cargo-wt-env)"
```

不要在当前 `main` 工作树上直接改 specs。

## 读（按序）

1. `llmanspec/AGENTS.md` → 「spec 约束层级」
2. `llmanspec/changes/c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`
3. `llmanspec/changes/c2210-update-specs-product-level/research/audit-table.md`（判定尺子 + 批次 1 草稿 + 文末 10 条）

尺子：**实现能改、对外行为不变 → 不进 spec。**

| 判定 | 本切片动作 |
|---|---|
| `keep` | 不动 |
| `rewrite-product` | 按表内草稿改写成 WHAT；删路径/函数名/行号 |
| `move-to-AGENTS` | spec 里删或改成一句指针；**对应 AGENTS.md 补一句**（根或 `src/AGENTS.md` / `src/app/tui/AGENTS.md`，选最近的）。禁止把长教程搬进 AGENTS |
| `delete` | 删该 req（及仅服务于它的 feature 场景） |

大组织方向可留：分层依赖、端口 seam、crate 边界、组合根、跨面同源。

## 本切片范围（按摩擦顺序，必须做完 1–8；9–10 有余力再做）

1. **ath12 + qg06 + tests.rs 复杂度闸双写**：spec 不钉文件名/行数/圈复杂度数字；迁体量策略到 `src/AGENTS.md`（若那里已有体量段，只加指针，不复制长文）。`test-qa-gate` qg06 若只是「qa 跑复杂度脚本」可 keep 脚本名作为闸契约，但不要钉 `host/mod.rs` 路径。
2. **layer-architecture 路径钉**：按批次 1 草稿改 la1/2/4/6/7/15/17/19/20；la9/la21/la-dispatch-consume 迁 AGENTS；**la14 删**。
3. **atc4/atc26 + adp\***：改成「单一书面视觉意图 SSOT + 禁止第二套实现当完成条件」。**不要**在本切片删除 playground 目录或 HTML。adp10 过期 Next-wave → 删。adp 里钉 `index.html` 路径的改为「生成物/静图闸存在」（adp1 类生成物不变量可留）。
4. **tt02**：删过期行号与「重建第二实例」变通句（已被 `TuiTestHarness` 取代）。保留可观察的测试分层语义。
5. **BDD 元闸 req 族**：a10/s8/s17/t13/t16/c6/ar-pilot 及句尾「MUST 有可执行 BDD 场景」→ 删（由 test-bdd/CI 管）。
6. **防复活**：la14、ce11、r71、rc1 防复活半句、ar13 → 删（先 `rg` 确认对象不存在）。
7. **valid_scope**：本切片涉及的 cap 把文件路径钉放宽为层/包范围（产品 TUI 面、agent 层、xylitol-tui 包…）。不要一次改 65 个 cap 的 valid_scope，只改你动到的那些。
8. **ath10**：整条迁面 AGENTS（布局地图本就是 `src/app/tui/AGENTS.md` 职责）；feature 里「打开 AGENTS.md」场景删或改成行为断言。

有余力：

9. ux1–ux4、ed03、tp01、atc2/atc24 等函数名钉 → 「或等价入口」。
10. 歧义补钉：ath6 同序验证口径、atc19 NextTurn 消费、ath24 Loader 帧 vs 上区缓存、avs1 不要钉 H1–H9 测试函数名。

## 禁止

- 在默认分支改 `llmanspec/specs/**`
- 改产品 Rust（除非 AGENTS 指针必须动的文档）
- 删 playground / DESIGN.md
- 把代码组织细节继续留在 spec 里「以免以后忘」
- 新写兼容别名 req

## 验证

- 改完的 capability 仍能被 llman/spec 校验解析（不要弄坏 toon/frontmatter）
- `rg 'src/app/tui/host/mod.rs' llmanspec/specs` 等路径钉：你动过的 cap 里应显著减少
- 不要跑整仓 `just qa` 当本切片完成条件（specs 改了可能要 feature 对齐）；至少保证你改的 feature 场景步骤不再打开具体源文件当 Given

## 提交

分支 `c2210-spec-rewrite`。标题：`docs(spec): product-level rewrite for high-friction caps`。

Commit/PR 按摩擦 1–10 列清单：改了哪个 req、keep/rewrite/delete。不要 ff 进 main；等架构师对照 audit-table review。
