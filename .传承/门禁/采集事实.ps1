# 采集事实.ps1 —— 「会变的数字」唯一产地（事实快照）
#
# 存在意义：项目文档中的 members 数 / 测试数 / 门禁结果等**会变的数字**一旦手写，
# 就会随代码演进系统性失真（历史教训：文档声明 532 测试 / 37 members，实测 528 / 40）。
# 本脚本把这些数字改为**机器采集**，产出唯一事实源 `事实快照.json`，
# 并同时把**原始 stdout 落盘**（原始输出/ 目录）——任何结论可回溯到原始输出，
# 禁止仅以文档转述数字（见第 3 条治理要求）。
#
# 用法：
#   ./采集事实.ps1                 # 采集 members + 测试数（默认，不跑门禁）
#   ./采集事实.ps1 -含门禁          # 额外采集门禁结论（跑 总入口.ps1，耗时较长）
#   ./采集事实.ps1 -不跑测试        # 仅采集 members（离线/快速场景，测试数置 null）
#
# 产出：
#   .传承/门禁/事实快照.json          —— 结构化事实（时间戳/成员数/测试数/门禁）
#   .传承/门禁/原始输出/测试-最新.txt  —— cargo test 原始 stdout（含 test result 行）
#   .传承/门禁/原始输出/门禁-最新.txt  —— 门禁 总入口 原始 stdout（-含门禁 时）
param(
    [switch]$含门禁,
    [switch]$不跑测试
)

$ErrorActionPreference = 'Stop'
# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$输出目录 = Join-Path $PSScriptRoot '原始输出'
if (-not (Test-Path $输出目录)) { New-Item -ItemType Directory -Path $输出目录 -Force | Out-Null }
$快照路径 = Join-Path $PSScriptRoot '事实快照.json'
Set-Location $根

Write-Host '== 采集事实（会变的数字唯一产地）==' -ForegroundColor Cyan

# ---------- 1) members 数：从根 Cargo.toml 的 [workspace] members 数组实读 ----------
$成员数 = 0
$清单路径 = Join-Path $根 'Cargo.toml'
if (Test-Path $清单路径) {
    $原文 = Get-Content $清单路径 -Raw
    $匹配 = [regex]::Match($原文, 'members\s*=\s*\[(?<体>[\s\S]*?)\]')
    if ($匹配.Success) {
        $成员数 = ($匹配.Groups['体'].Value -split "`n" |
            Where-Object { $_.Trim() -ne '' -and -not $_.Trim().StartsWith('#') }).Count
    }
}
Write-Host "members = $成员数"

# ---------- 2) 测试数：cargo test --workspace 实测（原始 stdout 落盘） ----------
$测试数 = $null
$测试失败 = $null
if (-not $不跑测试) {
    $测试原始 = Join-Path $输出目录 '测试-最新.txt'
    cmd /c "cargo test --workspace --quiet > `"$测试原始`" 2>&1"
    # PowerShell 下 cargo 的 NativeCommandError 属误报，以退出码与 test result 行为准
    $退出码 = $LASTEXITCODE
    $行们 = Get-Content $测试原始 -ErrorAction SilentlyContinue
    $通过 = ($行们 | Select-String -Pattern 'test result: ok\. (\d+) passed' |
        ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
    $失败 = ($行们 | Select-String -Pattern 'test result: FAILED\. (\d+) passed; (\d+) failed' |
        ForEach-Object { [int]$_.Matches[0].Groups[2].Value } | Measure-Object -Sum).Sum
    if ($null -eq $通过) { $通过 = 0 }
    if ($null -eq $失败) { $失败 = 0 }
    $测试数 = [int]$通过
    $测试失败 = [int]$失败
    Write-Host "测试 = $测试数 通过 / $测试失败 失败（退出码 $退出码，原始输出：原始输出/测试-最新.txt）"
}

# ---------- 3) 门禁结论（可选）：跑 总入口.ps1 并落盘原始 stdout ----------
$门禁结论 = $null
if ($含门禁) {
    $门禁原始 = Join-Path $输出目录 '门禁-最新.txt'
    & (Join-Path $PSScriptRoot '总入口.ps1') *>&1 | Tee-Object -FilePath $门禁原始
    $门禁码 = $LASTEXITCODE
    $门禁行 = Get-Content $门禁原始 -ErrorAction SilentlyContinue
    $汇总 = $门禁行 | Select-String -Pattern '汇总：检查项 (\d+) / 失败 (\d+)' | Select-Object -Last 1
    $检查项 = $null
    $失败数 = $null
    if ($汇总) {
        $检查项 = [int]$汇总.Matches[0].Groups[1].Value
        $失败数 = [int]$汇总.Matches[0].Groups[2].Value
    }
    $门禁结论 = [ordered]@{
        退出码 = $门禁码
        检查项 = $检查项
        失败   = $失败数
        原始输出 = '原始输出/门禁-最新.txt'
    }
    Write-Host "门禁 退出码 $门禁码（原始输出：原始输出/门禁-最新.txt）"
}

# ---------- 4) 落盘事实快照（唯一事实源） ----------
$快照 = [ordered]@{
    生成时间   = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK')
    生成脚本   = '.传承/门禁/采集事实.ps1'
    members    = $成员数
    测试数     = $测试数
    测试失败数 = $测试失败
    测试原始输出 = if ($不跑测试) { $null } else { '原始输出/测试-最新.txt' }
    门禁       = $门禁结论
}
$快照 | ConvertTo-Json -Depth 5 | Set-Content -Path $快照路径 -Encoding UTF8
Write-Host "事实快照已写入：事实快照.json" -ForegroundColor Green
exit 0
