# Development Ownership(开发所有权)

## 并行工作流

- Worker A: spec(规格), task(任务), readiness(就绪), tree(监督树), registry(注册表) 和 child_runner(子任务运行器).
- Worker B: policy(策略), health(健康), shutdown(关闭), control(控制) 和 runtime(运行时).
- Worker C: event(事件), state(状态), journal(事件日志缓冲区), summary(摘要), observe(可观测性) 和 test_support(测试支持).
- Worker D: README(说明文档), examples(示例), manual(手册), docs(文档), scripts(脚本), artifacts(产物), CHANGELOG(变更日志), LICENSE(许可证), ASSUMPTIONS(假设记录) 和 FINAL_REPORT(最终报告).
- Lead agent(主代理): 集成 Subagents(子代理) 输出, 修复接口漂移, 补齐缺失测试, 运行最终验证和记录结果.
- public API(公开接口) 以 `specs/001-create-supervisor-core/contracts/public-api.md` 和当前可编译源码为一致性来源.

## 纠偏记录

- Worker B 的控制测试使用临时字符串规格启动 runtime(运行时). 已改为 `SupervisorSpec::root(Vec::new())`.
- Worker D 的 example(示例) 使用尚未落地的接口细节. 已实现 `ConfigState::to_supervisor_spec`, 并修正 `subscribe_events` 和 `RestartDecision` 用法.
- 部分模块测试缺少英文 module doc(模块文档). 已补齐.
- package include(打包包含清单) 过宽. 已改为仓库根锚定路径.

## 验证命令

```bash
cargo fmt --all --check
cargo check
cargo test
cargo check --examples
cargo doc --no-deps
scripts/check-coding-standard.sh
scripts/check-maintainability.sh
scripts/generate-sbom.sh
scripts/validate-sbom.sh
cargo package --list --allow-dirty
cargo package --allow-dirty
cargo publish --dry-run --allow-dirty
```

## 结论

并行工作流已经由 lead agent(主代理) 复核并集成. 当前验证结果通过.
