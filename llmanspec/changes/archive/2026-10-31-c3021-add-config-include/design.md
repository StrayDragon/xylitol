# 设计：配置 include

## 字段

- `AppConfig.include: Vec<String>`（serde default；schema 可见；示例配置注释说明）。

## 加载顺序

对每个配置层（全局 / 项目 / CLI）：

1. 读取并渲染该层。
2. 取出 `include` 列表（缺省空）。
3. 对每项：解析路径（相对包含文件目录；`~` → home），读取并渲染，递归处理其 `include`，再 `deep_merge` 到累加值。
4. 最后 `deep_merge` 当前层（覆盖同名字段）。

## 环检测

- 用规范化路径集合（`canonicalize`，失败则用原路径）记录已加载文件；重复出现即返回 `LoadError`（含链路）。

## 缺失文件

- `warn`（log）并跳过，不中断加载。

## 兼容

- 不写 `include` 的配置行为不变；`deny_unknown_fields` 仍生效（include 字段已注册）。
