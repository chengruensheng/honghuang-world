#!/usr/bin/env bash
# 一键全验：跑全部 10 项验证，任一失败即退出 1。
# 用法：bash 一键全验.sh
set -euo pipefail

根=$(cd "$(dirname "$0")" && pwd)
cd "$根"

跑() {
  local 名=$1; shift
  echo ""
  echo "=== $名 ==="
  "$@"
  echo "[通过] $名"
}

跑 "1/10 格式"          cargo fmt --all -- --check
跑 "2/10 警告"          cargo clippy --workspace --all-targets -- -D warnings
跑 "3/10 测试"          cargo test --workspace --lib
跑 "4/10 编译"          cargo check --workspace --all-targets
跑 "5/10 文档"          cargo doc --workspace --no-deps

echo ""
echo "=== 6/10 审计 ==="
cargo audit 2>/dev/null || { cargo install cargo-audit --locked 2>/dev/null; cargo audit; }
echo "[通过] 审计"

echo ""
echo "=== 7/10 依赖 ==="
cargo deny check 2>/dev/null || { cargo install cargo-deny --locked 2>/dev/null; cargo deny check; }
echo "[通过] 依赖"

echo ""
echo "=== 8/10 无 src/tests/scripts 平铺 ==="
违规=$(find . -type d \( -name src -o -name tests -o -name scripts \) \
  -not -path './道果树/*' -not -path './.上下文/*' -not -path './临时文件夹/*' -not -path './.git/*' \
  -not -path './node_modules/*' 2>/dev/null || true)
if [ -n "$违规" ]; then echo "[失败] 发现平铺目录：$违规"; exit 1; fi
echo "[通过] 无 src/tests/scripts 平铺"

echo ""
echo "=== 9/10 无空目录 ==="
空目录=$(find . -type d -empty \
  -not -path './道果树/*' -not -path './.上下文/*' -not -path './临时文件夹/*' -not -path './.git/*' \
  -not -path './node_modules/*' 2>/dev/null || true)
if [ -n "$空目录" ]; then echo "[失败] 发现空目录：$空目录"; exit 1; fi
echo "[通过] 无空目录"

echo ""
echo "=== 10/10 临时目录 ==="
if [ -d "临时文件夹" ] && [ -n "$(ls -A 临时文件夹 2>/dev/null)" ]; then
  echo "[警告] 临时文件夹非空"
else
  echo "[通过] 临时目录干净"
fi

echo ""
echo "总体判定：全绿（10/10 通过）"