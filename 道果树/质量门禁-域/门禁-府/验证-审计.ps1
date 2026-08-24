# 验证-审计：cargo audit（依赖漏洞）
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo audit 2>$null
if ($LASTEXITCODE -ne 0) {
    cargo install cargo-audit --locked 2>$null
    cargo audit
}
exit $LASTEXITCODE