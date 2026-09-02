---
depends_on:
  - c2475-add-serve-registration-discovery
---

# Host 回环凭据门禁（产品决策草案）

> 性质：**决策草案**。本票先记录问题与候补方向，待人类拍板「默认开 vs 配置开」后再
> propose 正式化；其中「凭据不进工具子进程 env」一条风险低、争议小，可先行单独落地。

## Why

当前监听器（127.0.0.1）**无任何鉴权**：同机任意进程/用户都能驱动 agent 读盘、执行命令。
个人单用户机器上这是真实攻击面（恶意 npm postinstall、CI 脚本等均可探测固定端口）。
writer lease 只闸「并发写身份」，不是访问控制。

通行做法是对本地回环也上凭据门禁——密码随机生成、跨重启稳定、
经发现契约携带给合法客户端；WS 等无法带 header 的场景用一次性 ticket 或查询参数兜底；
并有一条硬规则：**租约凭据不得出现在模型执行的工具子进程 env 里**
（这对 coding agent 尤其关键，见 research 笔记）。

## What Changes（意向，待拍板）

- 门禁形态：回环 Basic auth 或等价凭据闸；凭据跨重启稳定，经 c2475 注册文件携带给合法客户端。
- WS 升级场景的一次性 ticket / 查询参数兜底路径。
- **硬规则先行**：租约/凭据从工具子进程继承环境中剥离（独立小步，可先于门禁本体交付）。
- 未配置凭据时是否保持现状放行（个人开箱优先）vs 默认生成——核心拍板点。

## 非目标

- 不做多用户/远程公网部署的安全模型（远程体验另有边界）。
- 不引入完整账号体系。

## Impact

- `src/app/server/http.rs` 中间件位；工具执行路径的 env 剥离；
  c2475 注册文件 schema 扩展 password 字段。
- 拍板前不改任何行为。

## Further Notes

- 一手对照（外部实现的鉴权中间件、凭据脱敏规则摘录）：[research/credential-gate-notes.md](./research/credential-gate-notes.md)
