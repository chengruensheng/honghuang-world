# 校验空值.ps1 —— 检测空值条件边界（unwrap_or_default 无 is_empty 保护）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_empty_value.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "空值条件边界"
$violations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if ($line -match "添加规则|add_rule") {
            if ($line -match "vec!\[.*unwrap_or_default") {
                $violations += "$($_.FullName):$($i+1): 添加规则条件直接使用 unwrap_or_default"
            }
        }
        if ($line -match "let\s+(\w+)\s*=\s*vec!\[") {
            $condVar = $Matches[1]
            if ($line -match "unwrap_or_default") {
                if ($line -match "(\w+)\.unwrap_or_default") {
                    $srcVar = $Matches[1]
                    $hasProtection = $false
                    $start = [Math]::Max(0, $i - 10)
                    for ($j = $start; $j -lt $i; $j++) {
                        if ($lines[$j] -match "$srcVar.*is_empty|is_empty.*$srcVar") {
                            $hasProtection = $true
                            break
                        }
                    }
                    if (-not $hasProtection) {
                        $end = [Math]::Min($lines.Count - 1, $i + 15)
                        for ($j = $i + 1; $j -le $end; $j++) {
                            if ($lines[$j] -match "添加规则|add_rule" -and $lines[$j] -match [regex]::Escape($condVar)) {
                                $violations += "$($_.FullName):$($i+1): 条件变量 $condVar 中 $srcVar 来自 unwrap_or_default 且无空值保护，第$($j+1)行用作规则条件"
                                break
                            }
                        }
                    }
                }
            }
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"空值条件边界","passed":true,"evidence":"未发现无保护的空值条件边界问题","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"空值条件边界","passed":false,"evidence":"发现 ' + $violations.Count + ' 处无保护的空值条件边界问题","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
