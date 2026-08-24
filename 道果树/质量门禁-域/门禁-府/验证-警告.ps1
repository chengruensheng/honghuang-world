# 验证-警告：cargo clippy -D warnings
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
cargo clippy --workspace --all-targets -- -D warnings
exit $LASTEXITCODE