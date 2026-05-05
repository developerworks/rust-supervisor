# SBOM(软件物料清单) 验证

- Command(命令): `scripts/generate-sbom.sh`
- Result(结果): passed(通过)
- Command(命令): `scripts/validate-sbom.sh`
- Result(结果): passed(通过)
- Evidence(证据): CycloneDX JSON(CycloneDX JSON 格式) 和 SPDX JSON(SPDX JSON 格式) 文件存在并通过格式校验.
- Failure record(失败记录): 并行运行生成和校验时发生一次读写竞态, 已改为顺序执行后通过.
