# 策略模型

## 监督策略

`SupervisionStrategy`(监督策略)决定失败后的重启范围. `OneForOne`(一对一)只选择失败 child(子任务). `OneForAll`(一对全部)选择同组全部 child(子任务). `RestForOne`(从失败处开始)选择失败 child(子任务)和之后声明的 child(子任务).

`restart_scope` 根据 `SupervisorTree`(监督树), 策略和失败 child id(子任务标识)计算重启范围.

runtime control loop(运行时控制循环) 现在会接收 child exit(子任务退出),并在 policy(策略) 返回重启决策时自动执行选定的 supervision strategy(监督策略).

## 重启策略

`RestartPolicy`(重启策略)包含 `Permanent`(永久), `Transient`(瞬时)和 `Temporary`(临时). `PolicyEngine`(策略引擎)读取 `TaskExit`(任务退出), 失败类别和重启策略, 输出 `RestartDecision`(重启决策).

## 退避和抖动

`BackoffPolicy`(退避策略)描述初始延迟, 最大延迟, jitter(抖动)模式和 reset-after(稳定后重置). 测试可以使用 deterministic jitter(确定性抖动), 避免依赖随机结果.

## 熔断和隔离

`MeltdownPolicy`(熔断策略)限制一个窗口内的重启或失败次数. 超过 child-level fuse(子任务级熔断)会进入 quarantine(隔离). 超过 supervisor-level fuse(监督器级熔断)会升级到父级.

## 任务退出分类

`TaskExit`(任务退出)区分成功, 取消, 类型化失败, panic(恐慌)和 timeout(超时). 策略层必须读取类型化分类, 不应该从字符串推断行为.
