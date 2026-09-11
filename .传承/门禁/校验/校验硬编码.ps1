# 校验硬编码.ps1 —— 检测硬编码字符串（载荷键/文件路径/版本号）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_hardcode.ps1
# 归集：.传承/门禁/校验/（2026-09-11 落档 .传承结构规范）
param([string]$ProjectRoot = ".")
$name = "硬编码字符串"
$violations = @()

function Get-BraceDelta([string]$s) {
    $i = 0
    $len = $s.Length
    $d = 0
    while ($i -lt $len) {
        $ch = $s[$i]
        if ($ch -eq '/' -and ($i + 1) -lt $len -and $s[$i + 1] -eq '/') { break }
        if ($ch -eq '/' -and ($i + 1) -lt $len -and $s[$i + 1] -eq '*') {
            $j = $s.IndexOf('*/', $i + 2)
            if ($j -lt 0) { $i = $len } else { $i = $j + 2 }
            continue
        }
        if ($ch -eq "'") {
            $i++
            while ($i -lt $len) {
                if ($s[$i] -eq '\' -and ($i + 1) -lt $len) { $i += 2; continue }
                if ($s[$i] -eq "'") { $i++; break }
                $i++
            }
            continue
        }
        if ($ch -eq 'r' -and ($i + 1) -lt $len) {
            $k = $i + 1
            $hashCount = 0
            while ($k -lt $len -and $s[$k] -eq '#') { $hashCount++; $k++ }
            if ($k -lt $len -and $s[$k] -eq '"') {
                $close = $s.IndexOf('"', $k + 1)
                while ($close -ge 0) {
                    if ($close + $hashCount -lt $len -and $s.Substring($close + 1, $hashCount) -eq ('#' * $hashCount)) { break }
                    $close = $s.IndexOf('"', $close + 1)
                }
                if ($close -ge 0) { $i = $close + 1 + $hashCount } else { $i = $len }
                continue
            }
        }
        if ($ch -eq '"') {
            $i++
            while ($i -lt $len) {
                if ($s[$i] -eq '\' -and ($i + 1) -lt $len) { $i += 2; continue }
                if ($s[$i] -eq '"') { $i++; break }
                $i++
            }
            continue
        }
        if ($ch -eq '{') { $d++ }
        elseif ($ch -eq '}') { $d-- }
        $i++
    }
    return $d
}

Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    $inTest = $false
    $brace = 0
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if ($inTest) {
            $brace += (Get-BraceDelta $line)
            if ($brace -le 0) { $inTest = $false }
            continue
        }
        if ($line -match '#\[cfg\(test\)\]' -or $line -match '\bmod\s+tests\b') {
            $inTest = $true
            $brace = (Get-BraceDelta $line)
            continue
        }
        if ($line -match '^\s*//') { continue }
        if ($line -match '"(标签|标题|描述|版本|状态|类型|来源|时间|内容|名称)"' -and $line -notmatch "enum|match|serde|载荷键|PayloadKey|const|static|=> &\[") {
            $violations += "$($_.FullName):$($i+1): 硬编码载荷键字符串"
        }
        if ($line -match '"[A-Z]:\\|\.json"|\.toml"|\.md"' -and $line -notmatch "test|//|const|static|include_str") {
            $violations += "$($_.FullName):$($i+1): 硬编码文件路径"
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"硬编码字符串","passed":true,"evidence":"未发现硬编码字符串","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"硬编码字符串","passed":false,"evidence":"发现 ' + $violations.Count + ' 处硬编码字符串","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
