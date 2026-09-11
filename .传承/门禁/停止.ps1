# 停止.ps1 —— 洪荒·世界 后端一键停止（PowerShell）
# 用法：
#   ./停止.ps1            终止监听 8321 端口的后端进程
# 说明：
#   * 通过 netstat 找到占用 8321 端口的 PID，taskkill /F /T 终止
#   * 幂等可重入：端口未被占用时静默退出
$ErrorActionPreference = 'SilentlyContinue'
# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $根
Write-Host '== 洪荒·世界 后端一键停止 ==' -ForegroundColor Cyan

$端口 = 8321
$占用 = netstat -ano | findstr ":${端口}"
if (-not $占用) {
    Write-Host "[1/1] 端口 ${端口} 未被占用，无后端运行。" -ForegroundColor Yellow
    exit 0
}

# 解析 PID（最后一列，且非自身 PID）
$自身 = $PID
$PIDs = $占用 | ForEach-Object {
    $cols = $_ -split '\s+'
    if ($cols.Count -ge 5) { $cols[-1] }
} | Where-Object { $_ -match '^\d+$' -and $_ -ne "$自身" } | Sort-Object -Unique

if (-not $PIDs) {
    Write-Host "[1/1] 未找到可终止的进程。" -ForegroundColor Yellow
    exit 0
}

foreach ($p in $PIDs) {
    Write-Host "[1/1] 终止 PID $p ..."
    taskkill /F /T /PID $p | Out-Null
}

Start-Sleep -Milliseconds 500
$再查 = netstat -ano | findstr ":${端口}"
if ($再查) {
    Write-Host "[1/1] 端口 ${端口} 仍被占用，请人工核查。" -ForegroundColor Red
    exit 1
}
Write-Host "[1/1] 后端已停止。" -ForegroundColor Green
