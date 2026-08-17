# 派工 prompt：死代码分诊表（c2220）

把下面整段交给另一个 agent。只出分诊表，**默认不删代码**（除非表里标了「无争议真死」且你在 review 里放行）。

---

你在 xylitol 的 **独立 git worktree**。

```bash
eval "$(just cargo-wt-env)"
```

读：

- `.claude/skills/audit-dead-code/SKILL.md`（三类：真死 / 逻辑死 / 预留）
- 根 `AGENTS.md`「Pre-0.0.1 卫生」
- `llmanspec/changes/c2220-update-pre-release-hygiene/research/pre-release-hygiene.md`

## 做

1. `rg -n '#\[allow\(dead_code\)\]' src packages --type rust` 全表。
2. 对每个压制点：符号、文件、为何 allow、三类判定、建议（删 / 激活 / 留+注释落地条件）。
3. 抽查 skill 里的已知热点是否仍真：`XyRemoteDriver`、`app/gui.rs`、terminal.rs allow。以 **现时代码** 为准，不要抄 skill 旧表当结论。
4. **不要** 临时加 `#![deny(dead_code)]` 后把改动提交；若做了实验编译，结束时撤回。

## 交付

覆盖写：

`llmanspec/changes/c2220-update-pre-release-hygiene/research/dead-code-triage.md`

```text
| 路径 | 符号 | 判定 | 建议 | 依据（入口链或零引用） |
```

文末：建议本 wt **可以立刻删** 的无争议真死清单（≤15 项）；拿不准的单独一节。

## 禁止

- 默认分支改生产代码或 live specs
- 把逻辑死当成预留长期留
- 为过编译再加新的 `allow(dead_code)`

完成：表覆盖全部 `allow(dead_code)` 命中；feature 分支 commit `docs(sdd): dead-code triage table`。
