# 并行治理

## 并行范围

实现阶段按 Worker A, Worker B, Worker C 和 Worker D 拆分. lead agent(主代理) 负责复核 Subagents(子代理) 输出, 修复接口漂移, 补齐缺失测试和运行最终验证.

## 协作规则

- 以 `specs/001-create-supervisor-core/tasks.md` 作为任务边界.
- 以 `specs/001-create-supervisor-core/quickstart.md` 作为验证路径.
- 以 `specs/001-create-supervisor-core/contracts/public-api.md` 作为公开 API(接口) 名称来源.
- 示例跟随最终 API(接口) 名称, 不增加 compatibility export(兼容导出).
- 发现 compile drift(编译偏移), API drift(接口偏移) 或 documentation drift(文档偏移) 时, lead agent(主代理) 必须在同一轮集成中纠偏.

## 完成证据

完成证据记录在 `artifacts/validation/documentation-ownership.md`. 该文件说明并行工作流, 纠偏记录和验证命令.
