#!/usr/bin/env sh
# 校验 docs/tutorials/ 的教程结构(与 core/tests/tutorials.rs 相同规则):
#   - 至少 30 篇 .md 教程
#   - 每篇有 `# ` 标题、至少一个 `## ` 小节、至少 20 行内容
# 用法:sh tools/check-tutorials.sh [docs/tutorials 目录,默认仓库内]
set -eu

DIR="${1:-$(dirname "$0")/../docs/tutorials}"
MIN_COUNT=30
MIN_LINES=20
FAIL=0

if [ ! -d "$DIR" ]; then
    echo "错误:教程目录不存在:$DIR"
    exit 1
fi

COUNT=0
for f in "$DIR"/*.md; do
    [ -f "$f" ] || continue
    COUNT=$((COUNT + 1))
    LINES=$(wc -l < "$f")
    H1=$(grep -c '^# ' "$f" || true)
    H2=$(grep -c '^## ' "$f" || true)
    if [ "$LINES" -lt "$MIN_LINES" ] || [ "$H1" -lt 1 ] || [ "$H2" -lt 1 ]; then
        echo "FAIL:$(basename "$f") 行数=$LINES(需≥$MIN_LINES) h1=$H1(需≥1) h2=$H2(需≥1)"
        FAIL=1
    fi
done

if [ "$COUNT" -lt "$MIN_COUNT" ]; then
    echo "FAIL:教程数量 $COUNT < $MIN_COUNT"
    FAIL=1
fi

if [ "$FAIL" -ne 0 ]; then
    echo "教程结构校验失败"
    exit 1
fi
echo "OK:$COUNT 篇教程,结构全部合法($DIR)"
