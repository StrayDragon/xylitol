---
brief: "仓库瘦身盘点（examples / scripts / 子包调用方交叉、删留闸、一次性 lab 进 freeze research）"
---

# 仓库瘦身盘点

本文件是 **how-to**：从树里拿掉不再服务产品 / 门禁 / 合约的叶子。不是进度板，不列本次删了谁。

## 何时用

盘 `examples/`、`scripts/`、子包（`crates/` / `vendor/`）里「好像没人用」的文件。目标不清先对齐；行为合约变化走 SDD。

## 步骤

1. **盘点面**：`examples/`（根 + `crates/*/examples`）、`scripts/`、子包附件（误入库生成物、无 assert 的 lab 测试、孤立 recipe）。
2. **交叉调用方**（缺一不可）：`justfile` recipe、live `.feature`、`docs/`、包 `AGENTS.md`。只搜文件名不够，还要搜 `[[example]]` / `cargo run --example` / `qa-check-scripts` glob。
3. **工作表**（本地，不进 git）：路径 / 类 / 一句说明 / 删除影响 / 推荐（`删` / `留` / 推迟）。**推荐列是执行权威**：人改完后 agent **MUST** 按该列处理，不得用「事故门禁 / 以后重写」覆盖人的删留。
4. **分闸后再动**（下表）。`just qa`；触及真终端再加 `just qa-e2e`。工作表处理完即删，不留墓碑。

## 分闸

| 闸 | 例子 | 怎么做 |
|---|---|---|
| **现在删** | 无调用方的一次性 `lab_*`；无 assert 的 println 测试；误入库的 `llmanspec/.context`；人在推荐列写「删」的叶子 | 直接删。一次性 lab **有对应已归档 change 目录**时塞进该 dir 的 `research/` 再 `7z u` freeze 包，然后从树删除 |
| **validate-only** | spec / 研究文档点名了要删的路径 | 先改 live `.feature` 或文档指针，再删；`llman-sdd validate` |
| **must-SDD** | 用户能看见的面、默认二进制含什么、qa 组成语义变化 | 完整 change 生命周期。本闸管**行为合约**，不挡住工作表已拍板的删 |
| **留** | 人在推荐列写「留」；子包本身（`xylitol-tui` / 官方方言插件 / `vendor/gherkin`） | 只盘包内叶子 |

## 硬规则

- **推荐列权威**。agent 的默认倾向（留事故门禁、留设计预览壳）不得覆盖人已填的「删」。
- **一次性 lab → freeze `research/`**：对应 change 已在 `llmanspec/changes/archive/`（含 freeze 包内日期目录）时，把 `lab_*.rs` 放进该 change 的 `research/`，`7z u` 进 `freezed_changes.7z.archived`，再从工作树删除。找不到对应 change 才退回 git 历史。磁盘上只留 yaml 索引卡；日期目录在包内。
- **`scripts/check_*.py` 不是免死金牌**。compiler / clippy 已覆盖、或人判定临时 patch 的，可以删；复杂度一类「找工具 + 阈值」可内联进 justfile（钉 crates.io 尽可能新的 pin）。删后改掉钉路径的 live spec。
- **生成缓存**：`**/llmanspec/.context/` 进 gitignore；不要提交 pageindex。
- **覆盖已迁到单测 / BDD 的维护 lab**：可以删 example，但必须改掉 spec「由 lab_foo 覆盖」那句，指向仍存在的测试。
- **子包本身**不是瘦身对象；只盘包内叶子。
- **AGENTS / 活文档**：移除即改写，不留「曾／已退役」墓碑。

## 工作表列（复制即用）

```text
| 路径 | 类 | 说明 | 删除影响 | 推荐 |
```

推荐只填 `删` / `留` / 推迟原因。处理完删除该 markdown。
