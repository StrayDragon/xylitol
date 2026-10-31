# Tasks

## 1. 规则
- [ ] 新增 r3026（配置 include）

## 2. 实现
- [ ] `AppConfig.include` 字段与 schema
- [ ] loader 逐层处理 include（低优先级、相对路径、`~`、递归、环检测、缺失 warn）

## 3. 测试
- [ ] include 层合并与主文件覆盖
- [ ] 相对路径与 `~` 解析
- [ ] 缺失文件跳过且不失败
- [ ] 环检测报错

## 4. 门禁
- [ ] `cargo test -p xylitol-official-plugin-config` 绿
- [ ] `just qa` 绿
- [ ] 重新生成 `configs/example.yaml` 与 schema
