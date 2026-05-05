# Parallel Governance(并行治理)

## 并行范围

Implementation is split across Worker A, Worker B, Worker C, and Worker D. lead agent(主代理) reviews Subagents(子代理) output, fixes API drift(接口偏移), fills missing tests, and runs final validation.

## 协作规则

- task boundary(任务边界) 来自 `specs/001-create-supervisor-core/tasks.md`.
- validation path(验证路径) 来自 `specs/001-create-supervisor-core/quickstart.md`.
- public API(公开接口) 名称来自 `specs/001-create-supervisor-core/contracts/public-api.md`.
- example(示例) 只能跟随最终 API(接口), 不创建 compatibility export(兼容导出).
- 当出现 compile drift(编译偏移), API drift(接口偏移) 或 documentation drift(文档偏移) 时, lead agent(主代理) 必须在同一轮集成中纠偏.

## 完成证据

completion evidence(完成证据) 写入 `artifacts/validation/documentation-ownership.md`.
