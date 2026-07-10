



P0
- 1 我期望的是一次性完成, 我们流中 应该用 esc 进行停止, ctrl+c (这个主要是输入区为空之后才是这个行为) 如果输入区有内容, 首个ctrl+c 是清空输入区, 第二个ctrl+c 是退出, 副作用当然退出了
- 2 我需要 Alt+Enter follow-up 排队(等待当前agent loop全部完成), 直接 流中 Enter 发送的话, 是当前 AgentLoop 中 steer up 消息(紧接着一重循环的下一个追加)
- 3 我们模仿pi 的设计, 默认就是 yolo 模式, 因此所有操作都会默认执行 (只要用户 trust 这个目录), 我们简化这个审批流程是因为现在市场有很多很好用的沙盒了, 我们不模仿 claude code, codex 的 session 内逐个工具的审批确认流程, 但是我们需要保留hook能力 万一之后需要可以进行追加实现, 保留扩展点
- 4 ok
- 5 demo要实现对齐准 src/app/tui 能力, 可以参考 P0 1 中的说明与这个对齐
- 6 支持 我感觉可以直接调整对应 llmanspec/specs 中 packages-tui 和 老的app tui移除 避免我们碰到老的干扰
- 7 这个没有确定, 确实也是一个坑点 但是可以后续整理, 我认为我们可以更新 llmanspec/config.yml 说明 然后让 AGENTS.md 中说明对应目录的配置方式加深记忆  , 你可以本能的按照当前目录分类 比如 core-*, intra-*, 加上我们之前固定的 packages-tui-* 和 app-tui-* 这些specs命名 要求 另外 specs 应该为强制中文


P1
- 8 是的, 也要考虑全局看 哪些模块实现比较好 比较符合最佳实践, 我们要控制各个包复杂度, 尽可能不要越界, 各司其职, 包级开闭原则
- 9 这个 transcript 的等价设计, codex 是打开新的overlay界面 然后跳转, 我更想要的效果是pi的(两次esc唤出的树列表来跳转和 travel, 希望你仔细调研 ../pi 的能力, 应该需要 packages/xylitol-tui支持的
- 10 ok app tui 可以实现延后, 但是 我们需要agent_demo实现 提前验证功能,   mvp可以这样 之后可能调整, 但是需要有对应DESIGN.md 只不过mvp 延迟实现, 可以有对应的draft 提案 后置 (记住后置提案避免忘记和handoff交接导致失忆, 需要draft落地)
- 11 ok 保留能力 , app tui 可以实现延后, 但是 我们需要agent_demo实现 提前验证功能,  和10类似需要design.md 和 change draft 落地
- 12 app tui 可以实现延后, 但是 我们需要agent_demo实现 验证库功能
- 13 类似 9 我们需要agent_demo 实现, packages/xylitol-tui 库支持
- 14 这个mvp明确不支持, 我们仅保留 config.yaml 配置, 用户配置哪个即用哪个, 但是可以切换 models
- 15 双esc 类似 9 唤出对话历史对话树
- 16 这个可以延后之后设计, 预留好扩展空间即可, 可以在agent_demo 验证
- 17 ok
- 18 旧的代码之后移除, 包括specs 需要清理
- 19 这个需要提前验证处理
- 20 这个我感觉还是让 agent_demo 使用真实的和 src/app/tui 一样的库进行集成和验证 不要写假的



P2
- 21 ok  我支持, 另外考虑之前几次失败的经验, 我们能不能使用即时log? 用户可以自己tail watch 和tui运行打开的进度, 然后 AGENTS.md (tui 对应) 中有提示, 让人类和agent都能知道 默认打开 不需要控制, 但是 release 模式 是默认关闭即可
- 22 ok 我测试了 agent demo 除了极端情况 会直接抛异常报错卡死terminal emulater 其他 时候都可以自动resize, 这个可能需要增强体验 比如报错退出然后退出进程 不要卡死用户终端模拟器
- 23 重新思考下吧 这个当时可能有点问题, 你也可以参考 pi 实现 毕竟主体代码也是port过来的, 也可以进行适配处理
- 24 移除旧 tui 概念 重新思考
- 25 这个需要考虑 你来决定一个好的用户友好的合理的行为
- 26 ok
- 27 ok 一般都是行内(unified), 这个可以优化 比如用户宽度足够 我们可以自动切换为left-right-diff 模式
- 28 这个明确需要支持, 并且要高覆盖 我们最多用户就是 CJK 用户 包括奇怪的emoji等等文本
- 29 这个也是历史经验 需要借鉴 packages/xylitol-tui 的经验 并且在 app 级增强
- 30 ok
- 31 不 optional 了 所 有特性默认都打开 避免遗漏
- 32 请清理  **write-surface / audit-dead-code**  不需要的描述 或者写死的不合适的通用skill描述
- 33 ok
- 34 ok
- 35 ok 这个可能要优化下, 和 pi 一样 在首次打开没有trust 的目录时 弹出选项让用户选择后自动配置
- 36 需要保留 我之后想实现 server 在远程 但是 tui/web 控制远控
- 37 ok
- 38 ok

以上有的处理你不必亲历亲为 除了重点主线 比如 tui 设计与实现,  比如 重命名 specs 和规范化 你可以给我提示词写到临时目录用后即弃 我交给其他快速处理,  然后你审核即可
