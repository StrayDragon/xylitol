# c1585 Design

## Seam

`QueueMode::default` / `SteeringMode::default` 单测；配置 `unwrap_or_default` 经 SettingsManager。

## Change

```text
QueueMode::default = OneAtATime
SteeringMode::default = OneAtATime
# settings unset → get_steering_mode / get_follow_up_mode → OneAtATime
# explicit all still works
```

## Non-goals

产品 UI 热切换 mode。
