# 以 rutis 为内核的装配设计

## 三层插件

- **核心层**：kernel 直接 `root.plugin`，不进 loader rows；配置出现核心名 = 校验错误。别的插件只能 inject 或监听。
- **官方可替换层**：`Builtins` 注册，配置生成 rows。未配置 MCP/hooks 不装配。
- **第三方层（远期）**：同一 `Chain` 第二环。本次只留 admit 钩子（核心名拒绝 + 官方放行）。

## 热路径

`apply` 取一次 `Arc`，循环内不 `require`。每 token 不上总线。瀑布仅生命周期事件；禁 `parallel`。

## session = instance

每个 session 是 loader instanced group 的一份拷贝。会话私有服务用 `TypeKey::instance`。全局服务（config、provider、mcp 连接）留在根。

## MCP 首轮门

mcp 插件 `apply` 自带超时。tool-table `watch()` 等 Active/Failed，15s 超时放行。不用 injects 硬门控。

## 挂载准入

```
rows → admit(core-name-deny, official-allow) → Chain(official Builtins [, future resolver])
```

## 进口

sdk 再导出 rutis；kernel 依赖 `rutis-loader =0.13.0`。精确锁版。

## 权衡

- 不把 loop 做成可卸官方插件：单写者会丢。
- 不为第三方预埋清单/注册中心。
- `xylitol-kernel` 是组合根下沉后的执行/装配 crate，取代「根包 wiring」。
