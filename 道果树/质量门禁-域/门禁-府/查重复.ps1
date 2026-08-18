# 查重复：跨 .rs 文件检测重复代码块（5 行起），按块指纹统计重复行数。
# 判定：重复率 = 重复行 / 代码行，超过阈值（默认 15%，当前基线 10.6%，后续阶段逐步收紧至 5%）即失败。
# 用法：pwsh -File 查重复.ps1 [-阈值 0.15]
param([double]$阈值 = 0.15)

. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
$文件们 = 取源码文件 $根 '*.rs'
Write-Host "扫描 .rs 文件：$($文件们.Count) 个"

$块行数 = 5
# 指纹 → 出现过的文件路径集合（只统计跨文件重复，同文件内重复不算）
$指纹表 = @{}
$总代码行 = 0
$重复行 = 0

foreach ($文件 in $文件们) {
    $行们 = Get-Content $文件.FullName -Encoding UTF8
    # 规范化：去首尾空白、跳过空行与纯注释行
    $代码行 = @($行们 | ForEach-Object { $_.Trim() } | Where-Object {
        $_ -ne '' -and $_ -notmatch '^(//|///|#!|\*|/\*)'
    })
    $总代码行 += $代码行.Count
    for ($i = 0; $i -le $代码行.Count - $块行数; $i++) {
        $块 = $代码行[$i..($i + $块行数 - 1)] -join "`n"
        $指纹 = [System.BitConverter]::ToString(
            [System.Security.Cryptography.SHA256]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($块))
        )
        if (-not $指纹表.ContainsKey($指纹)) {
            $指纹表[$指纹] = @{ 文件 = $文件.FullName; 行数 = $代码行.Count }
        } else {
            $条目 = $指纹表[$指纹]
            if ($条目.文件 -ne $文件.FullName) {
                # 跨文件重复：首现文件与当前文件的该块行都算重复
                if (-not $条目.已计重复) { $重复行 += $块行数; $条目.已计重复 = $true }
                $重复行 += $块行数
                if ($条目.示例 -eq $null) {
                    $条目.示例 = "$($条目.文件) 与 $($文件.FullName) 重复块："
                    $条目.示例 += ($块 -split "`n" | Select-Object -First 3) -join ' / '
                }
            }
        }
    }
}

$重复率 = if ($总代码行 -gt 0) { $重复行 / $总代码行 } else { 0 }
Write-Host ("总代码行：{0}  重复行：{1}  重复率：{2:P1}" -f $总代码行, $重复行, $重复率)

# 输出前 5 个重复示例（便于人工核查）
$示例数 = 0
foreach ($条目 in $指纹表.Values) {
    if ($条目.已计重复 -and $示例数 -lt 5) {
        Write-Host "  [重复示例] $($条目.示例)"
        $示例数++
    }
}

if ($重复率 -gt $阈值) {
    Write-Host "[失败] 重复率 $('{0:P1}' -f $重复率) 超过阈值 $('{0:P1}' -f $阈值)"
    exit 1
} else {
    Write-Host "[通过] 重复率 $('{0:P1}' -f $重复率) 在阈值 $('{0:P1}' -f $阈值) 内"
    exit 0
}
