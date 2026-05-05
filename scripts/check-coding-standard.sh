#!/usr/bin/env sh
set -eu

fail() {
    printf '%s\n' "error: $1" >&2
    exit 1
}

require_file() {
    [ -f "$1" ] || fail "missing required file: $1"
}

check_no_chinese_punctuation() {
    if perl -CS -Mutf8 -ne 'if (/[，。；：！？、（）【】《》“”‘’]/) { print "$ARGV:$.:$_"; $found = 1 } END { exit($found ? 0 : 1) }' README.md README.zh.md ASSUMPTIONS.md FINAL_REPORT.md CHANGELOG.md manual/zh/index.md manual/en/index.md docs/zh/index.md docs/en/index.md docs/zh/quality-gates.md docs/en/quality-gates.md docs/zh/parallel-governance.md docs/en/parallel-governance.md artifacts/validation/documentation-ownership.md; then
        fail "Chinese punctuation is not allowed in documentation"
    fi
}

check_no_compatibility_language() {
    if grep -R -n -E 'compatibility wrapper|migration layer|deprecated facade|old API alias' README.md README.zh.md manual docs examples 2>/dev/null | grep -v '不提供\|不得\|禁止\|不描述\|No Compatibility'; then
        fail "unexpected compatibility language found"
    fi
}

require_file README.md
require_file README.zh.md
require_file ASSUMPTIONS.md
require_file FINAL_REPORT.md
require_file CHANGELOG.md
require_file LICENSE
require_file examples/config/supervisor.yaml
require_file examples/supervisor_quickstart.rs
require_file examples/config_tree_supervisor.rs
require_file examples/restart_policy_lab.rs
require_file examples/shutdown_tree.rs
require_file examples/observability_probe.rs
require_file manual/zh/index.md
require_file manual/en/index.md
require_file docs/zh/index.md
require_file docs/en/index.md
require_file docs/zh/quality-gates.md
require_file docs/en/quality-gates.md
require_file docs/zh/parallel-governance.md
require_file docs/en/parallel-governance.md
require_file artifacts/validation/documentation-ownership.md

check_no_chinese_punctuation
check_no_compatibility_language

printf '%s\n' "coding standard check passed"
