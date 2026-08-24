# 验证-文档：cargo doc --no-deps
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo doc --workspace --no-deps
exit $LASTEXITCODE