# cargo publish --dry-run(发布试运行) 验证

- Command(命令): `cargo publish --dry-run --allow-dirty`
- Result(结果): passed(通过)
- Evidence(证据): packaged 136 files, package(打包), verify(验证) 和 compile(编译) 通过, upload(上传) 因 dry run(试运行) 按预期中止.
