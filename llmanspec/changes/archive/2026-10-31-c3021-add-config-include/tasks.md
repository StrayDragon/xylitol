# Tasks

## 1. 规则
- [x] 新增 r3026（配置 include）

## 2. 实现
- [x] `AppConfig.include` 字段与 schema
- [x] loader 逐层处理 include（低优先级、相对路径、`~`、递归、环检测、缺失 warn）

## 3. 测试
- [x] include 层合并与主文件覆盖
- [x] 相对路径与 `~` 解析
- [x] 缺失文件跳过且不失败
- [x] 环检测报错

## 4. 门禁
- [x] `cargo test -p xylitol-official-plugin-config` 绿
- [x] `just qa` 绿
- [x] 重新生成 `configs/example.yaml` 与 schema
