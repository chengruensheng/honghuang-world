# 验证-编译：cargo check --all-targets
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo check --workspace --all-targets
exit $LASTEXITCODE