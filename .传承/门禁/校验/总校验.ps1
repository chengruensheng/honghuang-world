# 总校验.ps1 —— 洪荒·世界 AI 缺陷治理总入口（聚合 16 项专项检测）
# 用法：
#   ./总校验.ps1           顺序运行所有 校验<名>.ps1，对源码进行 16 类 AI 编码缺陷扫描
#   ./总校验.ps1 -Quick    只运行 P0 三项（硬编码 / 命名 / unwrap 滥用）
# 说明：
#   * 本脚本为入口脚本，逐项调用同目录下 校验<名>.ps1
#   * 任一项 exit 非零则整体失败
param(
    [switch]$Quick
)

$ErrorActionPreference = 'Stop'
# 项目根：本脚本位于 .传承/门禁/校验/，向上回溯 3 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$脚本目录 = $PSScriptRoot
Set-Location $根
Write-Host '== 洪荒·世界 AI 缺陷治理（总校验）==' -ForegroundColor Cyan

# 顺序：P0 → P1 → P2
$P0 = @('校验硬编码.ps1', '校验命名.ps1', '校验unwrap滥用.ps1')
$P1 = @(
    '校验空条件.ps1', '校验空值.ps1', '校验弱断言.ps1',
    '校验静默错误.ps1', '校验文档同步.ps1', '校验文档测试计数.ps1',
    '校验事实快照.ps1'
)
$P2 = @(
    '校验容器化误用.ps1', '校验自动持久化.ps1', '校验删除API防护.ps1',
    '校验前端测试覆盖.ps1', '校验生产含demo.ps1', '校验规则空条件.ps1'
)

$全部 = @($P0 + $P1 + $P2)
if ($Quick) {
    $全部 = $P0
    Write-Host '[快速模式] 仅跑 P0 三项' -ForegroundColor Yellow
}

$通过 = 0
$失败 = 0
foreach ($名 in $全部) {
    $路径 = Join-Path $脚本目录 $名
    if (-not (Test-Path $路径)) {
        Write-Host "[缺] $名 —— 未找到" -ForegroundColor Red
        $失败++
        continue
    }
    Write-Host "[跑] $名 ..."
    & $路径
    if ($LASTEXITCODE -eq 0) {
        $通过++
    } else {
        Write-Host "    ✗ $名 退出码 $LASTEXITCODE" -ForegroundColor Red
        $失败++
    }
}

Write-Host ""
Write-Host "== 汇总：通过 $通过 / 失败 $失败 =="
if ($失败 -gt 0) {
    Write-Host '== 失败 ==' -ForegroundColor Red
    exit 1
}
Write-Host '== 全部通过 ==' -ForegroundColor Green
exit 0
