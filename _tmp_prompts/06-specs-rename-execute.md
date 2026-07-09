# 提示词：按 SPECS_RENAME_MAP 批量重命名 specs（用后即弃）

> 仓库：`/home/l8ng/Projects/__straydragon__/xylitol`
> 映射表：`_tmp_prompts/SPECS_RENAME_MAP.md`
> **主线已批准执行 P0（及你确认的 P1）。先 freeze 再 rename，降低 archive 噪音。**

## 0. 前置（必须先做）

```bash
cd /home/l8ng/Projects/__straydragon__/xylitol

# 1) 工作树干净（除本任务外无杂改）
git status -sb

# 2) 冻结旧归档，减少 rename 时扫到的噪音（用户已计划）
llman sdd archive freeze --dry-run --keep-recent 5
# 确认候选后：
llman sdd archive freeze --keep-recent 5
# 若工具要求 --before：用今天或合适日期，例如
# llman sdd archive freeze --before 2026-07-10 --keep-recent 5

# 3) 全量校验基线
llman sdd validate --all --strict --no-interactive
llman sdd index rebuild
```

**禁止**：改 `src/` / `packages/` 业务代码；改 active purpose-draft 的 change id；动 `package-tui-*` / `app-tui-*`（已合规）。

---

## 1. 执行范围（本批）

### P0 — 批准执行（按表）

| 现名 | 新名 |
|---|---|
| `app-protocol` | `protocol-app` |
| `bash-execution` | `infra-bash` |
| `clipboard` | `infra-clipboard` |
| `git-utils` | `infra-git` |
| `image-utils` | `infra-image` |
| `network-config` | `infra-network` |
| `process-mgmt` | `infra-process` |
| `provider-adapter` | `infra-provider` |
| `hook-system` | `agent-hooks` |
| `prompt-template` | `agent-prompt` |
| `tool-system` | `agent-tools` |
| `session-persistence` | `agent-session-store` |
| `print-output` | `cli-print` |
| `compaction` | `domain-compaction` |
| `security-policy` | `domain-security` |

### P1 — 仅当你有把握时同批；否则停在映射表注释

优先可做：

| 现名 | 新名 |
|---|---|
| `bdd-tests` | `test-bdd` |
| `testing-standards` | `test-standards` |
| `fake-provider` | `test-fake-provider` |
| `provider-integration` | `test-provider-integration` |
| `model-registry` | `runtime-model-registry` |
| `resource-discovery` | `runtime-resource-discovery` |
| `diagnostics` | `infra-diagnostics` |
| `debug-log` | `infra-logging` |

**本批不要做**（需主线再议）：`architecture` / `layer-architecture` 合并、`workspace-structure`、`user-experience`、`build-config`。

**永不改名**：`app-tui`、`app-tui-*`、`package-tui-*`、`agent-runtime`、`agent-session`、`cli-entry`、`server-*`、`runtime-config`、`test-infra`。

---

## 2. 每个目录的机械步骤

对映射中每一对 `(OLD, NEW)`：

```bash
OLD=bash-execution
NEW=infra-bash

# a) 搬 main spec
git mv "llmanspec/specs/$OLD" "llmanspec/specs/$NEW"

# b) 改 spec.toon 内 name 字段（必须与目录名一致）
#    name: "bash-execution"  →  name: "infra-bash"

# c) 全局替换引用（谨慎，先 rg 再改）
rg -n "$OLD" llmanspec tests .agents AGENTS.md src --glob '!**/archive/**' || true
# 替换：
# - 其它 spec 的 feature_refs / 正文提到的 capability 名
# - active changes/*/specs/$OLD → git mv 到 $NEW，并改 delta 无 name 冲突
# - 文档/AGENTS 中的路径指针
# 不要改：changes/archive/**（已 freeze 的更不要碰）；git 历史

# d) 每批 5 个后校验
llman sdd validate --all --strict --no-interactive
```

**分批**：每批最多 5 个 rename → validate → commit。
建议 commit 信息：

```text
chore(specs): rename <old> to <new> (batch N)

Align capability directory/name with llmanspec layer prefixes.
```

或一批多个：`chore(specs): rename P0 infra/agent capability prefixes (batch 1)`。

---

## 3. Active changes 目录

```bash
# 若存在 llmanspec/changes/*/specs/$OLD
# 必须同步 git mv 到 $NEW，否则 validate 会挂
find llmanspec/changes -type d -name 'bash-execution'  # 对每个 OLD 检查
```

purpose-draft 里若只在 proposal 正文提到旧名，改为新名（可选但推荐）。

---

## 4. 中文（顺手，非阻塞）

本批改动的 `purpose` 若仍是英文，改为中文一句；**不要**借机重写全部 requirements 英文史。

---

## 5. 收尾

```bash
llman sdd validate --all --strict --no-interactive
llman sdd index rebuild
llman sdd list --specs | head -80

# 更新映射表：把已完成的行标 ✅，或删掉已完成行
# 写短 PR/提交说明：freeze 参数、完成的 OLD→NEW 列表、跳过的 P1
```

## 6. 失败时

- validate 报缺 spec / 重复 name → 检查 `name:` 字段与目录是否一致。
- 引用残留 → `rg OLD` 清掉。
- 不要 `--no-verify` 跳过 hook。
- 不要改 c450 已归档内容；若 c450 刚 archive，只动 `llmanspec/specs/` 合并后的文件。

## 完成标准

- [ ] freeze 已执行（或 dry-run 证明无可冻）
- [ ] P0 全部 OLD→NEW 完成，`name` 字段一致
- [ ] `validate --all --strict` 通过
- [ ] index rebuild
- [ ] `SPECS_RENAME_MAP.md` 已标注完成项
- [ ] 未触碰 `package-tui-*` / `app-tui-*` 业务实现
