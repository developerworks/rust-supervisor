#!/usr/bin/env sh
set -eu

fail() {
    printf '%s\n' "error: $1" >&2
    exit 1
}

require_file() {
    [ -f "$1" ] || fail "missing required file: $1"
}

require_pair() {
    require_file "docs/zh/$1"
    require_file "docs/en/$1"
}

example_count=$(find examples -maxdepth 1 -name '*.rs' -type f | wc -l | tr -d ' ')
[ "$example_count" -ge 5 ] || fail "expected at least five Rust examples"

require_file manual/zh/index.md
require_file manual/en/index.md
require_file docs/zh/index.md
require_file docs/en/index.md
require_pair quality-gates.md
require_pair parallel-governance.md
require_file artifacts/validation/documentation-ownership.md

if ! grep -R -q 'Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)' README.md README.zh.md manual docs artifacts/validation 2>/dev/null; then
    fail "missing required shutdown terminology"
fi

if ! grep -R -q 'rust-config-tree(集中配置树)' README.md README.zh.md manual docs examples/config/supervisor.yaml examples/*.rs 2>/dev/null; then
    fail "missing rust-config-tree terminology"
fi

printf '%s\n' "maintainability check passed"
