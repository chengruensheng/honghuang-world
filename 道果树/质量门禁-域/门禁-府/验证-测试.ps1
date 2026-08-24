# 验证-测试：cargo test --workspace --lib
. (Join-Path $PSScriptRoot '公共-函数.ps1')
Set-Location (找工作区根)
$env:WORLD_WORKSPACE_ROOT = $null
cargo test --workspace --lib
exit $LASTEXITCODE