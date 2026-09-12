# 总入口.ps1 —— IDE 启动前门禁统一入口（聚合所有检查型门禁）
# 用法：
#   ./总入口.ps1             跑全部检查门禁（当前 = 契约校验 16 项）
#   ./总入口.ps1 -Quick      快速模式（仅 P0 三项：硬编码 / 命名 / unwrap 滥用）
# 退出码：0 = 全部通过；非零 = 存在失败项（CI / IDE 可直接消费）
# 与「校验/总校验.ps1」的区别：总校验 = 校验这一类门的聚合；本脚本 = 所有检查型门的统一入口。
param(
    [switch]$Quick
)

$ErrorActionPreference = 'Stop'
# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $根

Write-Host '== 洪荒·世界 门禁总入口 ==' -ForegroundColor Cyan

$失败 = 0
$检查项数 = 0

# 1) 契约校验（15 项专项，聚合于 校验/总校验.ps1）
$总校验 = Join-Path $PSScriptRoot '校验\总校验.ps1'
$检查项数++
if (Test-Path $总校验) {
    if ($Quick) { & $总校验 -Quick } else { & $总校验 }
    if ($LASTEXITCODE -ne 0) { $失败++ }
} else {
    Write-Host '[缺] 校验\总校验.ps1 —— 未找到' -ForegroundColor Red
    $失败++
}

# 预留：后续新增检查门禁在此追加（如文档同步、构建验证等）

Write-Host ''
Write-Host "== 汇总：检查项 $检查项数 / 失败 $失败 =="
if ($失败 -gt 0) {
    Write-Host '== 门禁失败 ==' -ForegroundColor Red
    exit 1
}
Write-Host '== 门禁全部通过 ==' -ForegroundColor Green
exit 0
