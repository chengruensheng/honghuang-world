# 停止.ps1 —— 洪荒·世界 后端一键停止（PowerShell）
# 用法：
#   ./停止.ps1            终止监听 8321 端口的后端进程
# 说明：
#   * 只认 LISTENING 状态的监听套接字。旧版 `netstat | findstr ":8321"` 会把 TIME_WAIT
#     的连接行也算作「占用」，并从该行解析出 PID 0，进而对系统进程发起 taskkill（必被拒），
#     表现为「服务已停但脚本报端口仍被占用、反复失败」。
#   * 幂等可重入：端口无监听时静默退出。
$ErrorActionPreference = 'SilentlyContinue'
# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $根
Write-Host '== 洪荒·世界 后端一键停止 ==' -ForegroundColor Cyan

$端口 = 8321
$监听 = Get-NetTCPConnection -LocalPort $端口 -State Listen -ErrorAction SilentlyContinue
if (-not $监听) {
    Write-Host "[1/1] 端口 ${端口} 未被监听，无后端运行。" -ForegroundColor Yellow
    exit 0
}

# 解析 PID：仅取 LISTENING 行的 OwningProcess，排除 0（系统进程）与自身 PID
$自身 = $PID
$PIDs = $监听 | Select-Object -ExpandProperty OwningProcess |
    Where-Object { $_ -gt 0 -and $_ -ne $自身 } | Sort-Object -Unique

if (-not $PIDs) {
    Write-Host "[1/1] 未找到可终止的进程。" -ForegroundColor Yellow
    exit 0
}

foreach ($p in $PIDs) {
    Write-Host "[1/1] 终止 PID $p ..."
    taskkill /F /T /PID $p | Out-Null
}

Start-Sleep -Milliseconds 500
if (Get-NetTCPConnection -LocalPort $端口 -State Listen -ErrorAction SilentlyContinue) {
    Write-Host "[1/1] 端口 ${端口} 仍在监听，请人工核查。" -ForegroundColor Red
    exit 1
}
Write-Host "[1/1] 后端已停止。" -ForegroundColor Green
