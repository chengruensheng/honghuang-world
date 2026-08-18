# 检查-全部：质量门禁四件套（格式 + 静态检查 + 测试 + 重复检测）。
# 用法：pwsh -File 检查-全部.ps1 [-只查重复]
param([switch]$只查重复)

. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
Set-Location $根
Write-Host "工作区根：$根"

$失败 = $false

if (-not $只查重复) {
    Write-Host ""
    Write-Host "=== 1/4 格式检查（cargo fmt --check）==="
    cargo fmt --check
    if ($LASTEXITCODE -ne 0) { $失败 = $true; Write-Host "[失败] 格式未通过" }
    else { Write-Host "[通过] 格式一致" }

    Write-Host ""
    Write-Host "=== 2/4 静态检查（clippy -D warnings）==="
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { $失败 = $true; Write-Host "[失败] clippy 未通过" }
    else { Write-Host "[通过] clippy 零警告" }

    Write-Host ""
    Write-Host "=== 3/4 全量测试（cargo test --workspace --lib）==="
    cargo test --workspace --lib
    if ($LASTEXITCODE -ne 0) { $失败 = $true; Write-Host "[失败] 测试未通过" }
    else { Write-Host "[通过] 测试全绿" }
}

Write-Host ""
Write-Host "=== 4/4 重复检测（阈值 15%，当前基线 10.6%）==="
& (Join-Path $PSScriptRoot '查重复.ps1')
if ($LASTEXITCODE -ne 0) { $失败 = $true; Write-Host "[失败] 重复检测未通过" }
else { Write-Host "[通过] 重复检测达标" }

Write-Host ""
if ($失败) { Write-Host "总体判定：失败"; exit 1 }
else { Write-Host "总体判定：全绿"; exit 0 }
