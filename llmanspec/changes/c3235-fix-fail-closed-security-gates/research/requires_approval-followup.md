# requires_approval 死字段分析与审批业务评估（2026-11-11）

> 后续（2026-11-11 同日）：经对齐，逐项审批判定为需要的能力——已建 delayed draft c3330-add-hook-approval-gate（含防篡改要求与分层信任原则）；本期仍按本 change 任务移除死字段。本文的成本收益分析保留作为 c3330 的输入。

## 现状（已核证）

- 定义于 plugins/xylitol-official-plugin-hooks/src/config.rs:22，进 JSON schema。
- 全仓零读取：仅 tests/bdd/steps_domain_security.rs:606 与 steps_hooks.rs:13 两处夹具填 false。
- 结论：声明未接线的半截机制；「声明了 requires_approval: true」不会有任何行为差异。

## 它原本想解决什么

hooks 的 command 本身就是任意命令执行。项目层 hooks 来自项目 config——克隆一个恶意仓库并信任一次，其 hooks 即获得 RCE 面。逐钩子审批想防的是「我信任这个项目干活 ≠ 我审过它每条 hook 命令」。

## 现状的真实防线

项目层 hooks 随项目 config 加载，而项目 config 属项目本地资源、受 trust 门禁（未信任项目不加载；信任后 reload/restart 生效）。即现在只有一道粗闸（项目级），无细闸（钩子级）。

## 值不值（结论：当前不做）

成本：真审批 = 新状态机——钩子指纹（command 串哈希）、审批注册表持久化、命令变更即失效重批、审批 UX。相当于第二个 trust store，且正是本批清理的隐式状态机类型。
收益：防「信任项目但没审其 hooks」。当前产品形态（个人使用、配置自写、未分发）该场景概率低，且与项目信任决策高度重合。

## 加回的触发条件（书面化，届时走新 change）

1. hooks 配置可被分享 / 模板化 / 市场分发；
2. 团队共享 config 成为常见形态（成员无须逐人审 hooks）;
3. 出现真实安全事件或用户诉求。

## 届时的设计要点

- 复用 trust store 模式：指纹 → bool 注册表，独立于项目 trust（不叠加父目录继承）。
- 指纹 = 命令串哈希 + 事件模式；变更即失效。
- 审批在加载期（不在每次 dispatch），拒绝 = 该钩子不装配 + 启动诊断可见。

## 零成本可见性补偿（建议，非本 change 任务）

启动诊断输出「已装配 hooks：N 条（命令清单）」——把「装了什么」变成一眼可见，覆盖审批想解决的一半感知问题，且不引入任何新状态。
