# Tasks — c999-add-app-tui-model-arg-completion

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c999-add-app-tui-model-arg-completion --no-interactive`
- [x] 2. `UiRoot`：注册 SlashArg（无 bare）+ SlashCommand；`set_model_arg_catalog`
- [x] 3. host / 启动：拉取 `available_models` 填 catalog；OpenModels 路径同步刷新
- [x] 4. harness：`/model dee` Tab 写入；Esc 不改模型；无参 `/model` 仍 OpenModels
- [x] 5. `models-picker.md` 一行补全指针；`just fmt` + 相关 test 绿
