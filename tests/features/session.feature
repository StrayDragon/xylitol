# language: zh-CN
功能: 会话持久化
  作为一个 LLM 代理平台
  我想要持久化会话
  以便跨进程重启保留对话历史

  场景: 创建并加载会话
    假定 会话存储目录已初始化
    当 创建一个新会话 "test-session"
    并且 向会话追加一条消息 "用户问好"
    并且 加载会话 "test-session"
    那么 会话包含 1 条记录
    并且 记录类型为 "message"

  场景: 会话列表
    假定 存在会话 "session-a"
    并且 存在会话 "session-b"
    当 列出所有会话
    那么 结果包含 "session-a"
    并且 结果包含 "session-b"

  场景: 会话分叉
    假定 存在会话 "parent" 包含 10 条记录
    当 在记录 5 处分叉创建会话 "child"
    那么 会话 "child" 包含 5 条记录
    并且 会话 "child" 包含一个 branch_summary 记录

  场景: 会话树导航
    假定 存在会话树: "root" → "branch-a" → "branch-b"
    当 导航到 "branch-b"
    那么 上下文包含 branch-a 和 branch-b 的摘要

  场景: 模型切换记录
    假定 存在会话 "model-test"
    当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
    那么 会话包含 model_change 记录
    并且 model_change 记录显示 provider 为 "anthropic"

  场景: 思考级别切换记录
    假定 存在会话 "think-test"
    当 切换思考级别为 "high"
    那么 会话包含 thinking_level_change 记录

  场景: JSONL 文件格式正确
    假定 存在会话 "format-test"
    当 向会话追加 3 条不同类型的记录
    那么 JSONL 文件每行是一个完整的 JSON 对象
    并且 第一行包含 version 字段

  场景: 为会话条目设置标签
    假定 存在会话 "label-test"
    当 向会话追加一条消息 "标记我"
    并且 为最后一条记录设置标签 "重要"
    那么 该记录的标签为 "重要"

  场景: 清除会话条目标签
    假定 存在会话 "label-clear-test"
    当 向会话追加一条消息 "标签测试"
    并且 为最后一条记录设置标签 "重要"
    并且 清除该记录的标签
    那么 该记录没有标签
