# 验证-依赖：cargo deny check（许可协议 + 重复依赖 + 源码）
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo deny check 2>$null
if ($LASTEXITCODE -ne 0) {
    cargo install cargo-deny --locked 2>$null
    cargo deny check
}
exit $LASTEXITCODE