#!/usr/bin/env bash
# 检查 src/ 下 Rust 源码中 loop 块是否在 50 行内含有 yield 点
# 非阻断告警, CI 中仅输出警告信息
set -uo pipefail

src_dir="src"
violations=0

while IFS= read -r file; do
    # 找到 loop { 的行号
    while IFS=: read -r line_no content; do
        # 检查接下来 50 行内是否有 yield_now() 或 .await
        if ! tail -n +"$line_no" "$file" | head -n 50 | grep -q 'yield_now\|\.await'; then
            echo "WARNING: $file:$line_no: loop without yield point in next 50 lines"
            ((violations++))
        fi
    done < <(grep -n 'loop\s*{' "$file" || true)
done < <(find "$src_dir" -name '*.rs' -type f)

if [ "$violations" -gt 0 ]; then
    echo "⚠️  Found $violations loop(s) without yield points (non-blocking)"
fi
