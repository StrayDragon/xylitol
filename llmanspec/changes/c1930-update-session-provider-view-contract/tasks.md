# Tasks: c1930-update-session-provider-view-contract

> Specs landing 须在 `change start` / attach 之后。规划壳可先在默认分支补齐。

## 0. Review 门

- [x] 0.1 主目标 = resume/import 后 API **前缀**与同进程续跑一致（为 cache）；不迁 ResponseItem 史
- [x] 0.2 **本波不做**状态栏 / `AgentStatusBar` / 导出 strip 产品
- [x] 0.3 `landing.tmp.md`：字段级前缀（P0–P5）+ 漂移清单 + lab/Langfuse 实验臂
- [x] 0.4 衔接 c1890 / c1925 / c1900（tools）/ c1905（date 声明）
- [x] 0.5 离线单测意向 + 在线 lab（`lab_session_prefix_idempotency`；不进 qa）；不强制新 BDD step
- [x] 0.6 Lab 首跑+复跑：offline/import 哈希相等；arm_b ≥ arm_a；Langfuse 抽样同形（landing §5）
- [x] 0.7 Branch binding（`change start` → `sdd/c1930-…`）

## 1. Specs landing（Branch binding 后）

- [x] 1.1 `package-ai-bridge`：`pab27` assemble 前缀幂等（衔 pab15/17/24/25）
- [x] 1.2 `agent-session`：`as48` resume/import → 投影唯一；稳定折叠文案
- [x] 1.3 **跳过** AgentStatusBar 盘面 req
- [x] 1.4 指针：date→c1905；冻表→c1900；冻结替换→c1910（写在 as48/pab27 statement）
- [x] 1.5 validate specs（`--no-check`）+ `readyToImplement=true`

## 2. Apply

- [x] 2.1 离线 golden：fixture → project → assemble 两次哈希相等
- [x] 2.2 JSONL import 形（parse→投影→assemble）与内存史相等
- [x] 2.3 reasoning 序 + 折叠文案钉死
- [x] 2.4 Lab：`lab_session_prefix_idempotency`（live-provider）；Langfuse dump 同形对照（landing §5）

## 3. 文档

- [x] 3.1 research §5.3 交叉链；landing 随 change 保留

## 4. 校验

- [x] 4.1 `change start` → specs land → validate specs
- [x] 4.2 apply 单测 + lab 证据
- [ ] 4.3 verify → archive
