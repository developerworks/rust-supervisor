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

require_manual_pair() {
    require_file "manual/zh/$1"
    require_file "manual/en/$1"
}

require_manual_language_switch() {
    page="$1"
    html_page="${page%.md}.html"
    grep -F -q "语言: [English](../en/$html_page)" "manual/zh/$page" || fail "missing zh manual language switch: $page"
    grep -F -q "Language: [中文](../zh/$html_page)" "manual/en/$page" || fail "missing en manual language switch: $page"
}

require_manual_summary_language_switch() {
    grep -F -q -- "- [English Manual](language.md)" manual/zh/SUMMARY.md || fail "missing zh manual summary language switch"
    grep -F -q -- "- [中文手册](language.md)" manual/en/SUMMARY.md || fail "missing en manual summary language switch"
    grep -F -q '<meta http-equiv="refresh" content="0; url=../en/index.html">' manual/zh/language.md || fail "missing zh manual language redirect"
    grep -F -q '<meta http-equiv="refresh" content="0; url=../zh/index.html">' manual/en/language.md || fail "missing en manual language redirect"
}

require_file .github/dependabot.yml
require_file .github/workflows/pages.yml
require_file book.toml
require_file manual/theme/left-aligned.css
require_file scripts/publish-pages.sh
require_manual_pair book.toml
require_manual_pair SUMMARY.md
require_manual_pair language.md
require_manual_summary_language_switch

example_count=$(find examples -maxdepth 1 -name '*.rs' -type f | wc -l | tr -d ' ')
[ "$example_count" -ge 9 ] || fail "expected at least nine Rust examples"

for page in index.md getting-started.md configuration.md supervisor-tree.md task-model.md policies.md runtime-control.md dashboard.md shutdown.md observability.md examples.md quality-gates.md; do
    require_manual_pair "$page"
    require_manual_language_switch "$page"
done
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
