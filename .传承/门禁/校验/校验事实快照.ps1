# 校验事实快照.ps1 —— 事实快照门禁（会变的数字必须有出处、且未过期）
#
# 检查项（任一不满足即 FAIL）：
#   1. 快照存在（缺 → 提示先跑 采集事实.ps1，杜绝「文档有数字但无出处」）
#   2. 快照未过期（默认 24 小时内；陈旧快照 = 陈旧数字）
#   3. 快照 members 与根 Cargo.toml 实读一致（防快照与源码脱节）
#   4. 快照中每个数字结论都有**原始输出文件**支撑（第 3 条：结论不得仅由文档转述）
#
# 与 校验文档同步.ps1 / 校验文档测试计数.ps1 的分工：那两项比对「文档声明 vs 直接实测」，
# 本项保证「机器采集的事实源」新鲜、自洽、可回溯——数字的唯一种子源。
param([string]$ProjectRoot = ".")

$ErrorActionPreference = 'Stop'
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$门禁目录 = Join-Path $根 '.传承\门禁'
$快照路径 = Join-Path $门禁目录 '事实快照.json'
$名 = '事实快照'

function 输出($通过, $证据, $细节) {
    $对象 = [ordered]@{ name = $名; passed = $通过; evidence = $证据; details = $细节 }
    Write-Output ($对象 | ConvertTo-Json -Compress -Depth 5)
}

if (-not (Test-Path $快照路径)) {
    输出 $false '事实快照缺失：会变的数字无机器来源。请运行 .传承/门禁/采集事实.ps1 生成' @{}
    exit 1
}

try {
    $快照 = Get-Content $快照路径 -Raw | ConvertFrom-Json
} catch {
    输出 $false "事实快照不可解析：$($_.Exception.Message)" @{}
    exit 1
}

# 2) 新鲜度：陈旧快照等同陈旧数字
$生成时间 = $null
try { $生成时间 = [datetime]$快照.生成时间 } catch { }
if ($null -eq $生成时间) {
    输出 $false '事实快照缺「生成时间」，无法判定新鲜度' @{}
    exit 1
}
$年龄小时 = [math]::Round(((Get-Date) - $生成时间).TotalHours, 1)
if ($年龄小时 -gt 24) {
    输出 $false "事实快照已过期 $年龄小时 小时（> 24）：请重跑 采集事实.ps1" @{ 年龄小时 = $年龄小时 }
    exit 1
}

# 3) members 与源码实读一致：快照不得与 Cargo.toml 脱节
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
if ([int]$快照.members -ne $成员数) {
    输出 $false "快照 members $($快照.members) != Cargo.toml 实读 $成员数：请重跑 采集事实.ps1" @{ 快照 = $快照.members; 实读 = $成员数 }
    exit 1
}

# 4) 数字必须有原始输出支撑：结论可回溯到原始 stdout，不接受「只有数字没有证据」
$缺失证据 = @()
if ($null -ne $快照.测试数 -and $快照.测试数 -ne '') {
    $证据相对 = [string]$快照.测试原始输出
    if ([string]::IsNullOrWhiteSpace($证据相对)) {
        $缺失证据 += '测试数（未声明 测试原始输出）'
    } else {
        $证据路径 = Join-Path $门禁目录 ($证据相对 -replace '/', '\')
        if (-not (Test-Path $证据路径)) { $缺失证据 += "测试数（原始输出文件不存在：$证据相对）" }
    }
}
if ($null -ne $快照.门禁 -and $快照.门禁.退出码 -ne $null) {
    $证据相对 = [string]$快照.门禁.原始输出
    if ([string]::IsNullOrWhiteSpace($证据相对)) {
        $缺失证据 += '门禁（未声明 原始输出）'
    } else {
        $证据路径 = Join-Path $门禁目录 ($证据相对 -replace '/', '\')
        if (-not (Test-Path $证据路径)) { $缺失证据 += "门禁（原始输出文件不存在：$证据相对）" }
    }
    # 注意：此处**不**以「快照记录的门禁结论是否为 0」作为本门禁的通过条件。
    # 那会形成自指循环——本轮门禁运行的结果先写回快照，下轮又据此判定本轮，
    # 一旦某轮失败便永远失败。门禁结论仅作审计记录（附于 evidence），可回溯即可。
}
if ($缺失证据.Count -gt 0) {
    输出 $false ("数字缺原始输出支撑：" + ($缺失证据 -join '；')) @{ 缺失 = $缺失证据 }
    exit 1
}

输出 $true "快照新鲜（$年龄小时 小时内）· members $成员数 · 测试 $($快照.测试数) · 原始输出可回溯" @{
    生成时间 = [string]$快照.生成时间
    members = $成员数
    测试数 = $快照.测试数
}
exit 0
