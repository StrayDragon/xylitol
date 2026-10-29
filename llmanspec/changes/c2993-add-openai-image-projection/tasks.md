# Tasks

- [ ] 1. **specs**（infra-provider）：新增 `r2975 openai-image-projection` 与两条场景。测：`llman-sdd validate c2993-add-openai-image-projection`。
- [ ] 2. **provider**（native）：新增共享 `image_data_url`；Responses user/tool 投影 `input_image`；Completions user `image_url` 与 tool 图片批量补发 user 消息。测：`cargo test -p xylitol-official-plugin-provider-openai`。
- [ ] 3. **tests**（repo）：插件单测覆盖用户图、工具图、文本工具结果不变；BDD 场景步骤/绑定。测：`cargo test --all-features --test bdd r2975`。
- [ ] 4. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
