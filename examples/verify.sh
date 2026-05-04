#!/bin/sh
# Verify every examples/*.sql produces its matching *.expected.
# Run from the repo root:  sh examples/verify.sh

set -eu

BIN="${BIN:-./target/release/loco-generate-scaffold-via-sql-schema}"

if [ ! -x "$BIN" ]; then
    echo "error: binary not found at $BIN" >&2
    echo "       build with: cargo build --release" >&2
    echo "       or set BIN=path/to/binary" >&2
    exit 2
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

fail=0
for sql in examples/*.sql; do
    expected="${sql%.sql}.expected"
    base=$(basename "$sql" .sql)

    # Pick dialect from filename suffix.
    case "$base" in
        *-mysql)    dialect=mysql ;;
        *-sqlite)   dialect=sqlite ;;
        *)          dialect=postgres ;;
    esac

    "$BIN" -d "$dialect" < "$sql" > "$tmp" 2>/dev/null

    if cmp -s "$tmp" "$expected"; then
        printf "ok    %s\n" "$base"
    else
        printf "FAIL  %s\n" "$base" >&2
        diff "$expected" "$tmp" >&2 || true
        fail=$((fail + 1))
    fi
done

if [ "$fail" -gt 0 ]; then
    echo "" >&2
    echo "$fail example(s) failed" >&2
    exit 1
fi
echo ""
echo "all examples match"
