# 验证-格式：cargo fmt --check
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo fmt --all -- --check
exit $LASTEXITCODE