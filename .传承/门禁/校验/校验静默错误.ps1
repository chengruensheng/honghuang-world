# 校验静默错误.ps1 —— 检测静默吞错（let _ = / if let Ok(_) / unwrap_or）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_silent_error.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "静默吞错"
$violations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "let\s+_\s*=" -and $lines[$i] -match "\?|Result|Err|error") {
            $violations += "$($_.FullName):$($i+1): let _ = 忽略错误"
        }
        if ($lines[$i] -match "if\s+let\s+Ok\(_\)" -and $lines[$i] -notmatch "else|warn|error|log") {
            $violations += "$($_.FullName):$($i+1): if let Ok(_) 不处理错误"
        }
        if ($lines[$i] -match "\.unwrap_or\(" -and $lines[$i] -notmatch "default|0|false|空") {
            $violations += "$($_.FullName):$($i+1): unwrap_or 静默降级"
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"静默吞错","passed":true,"evidence":"未发现静默吞错","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"静默吞错","passed":false,"evidence":"发现 ' + $violations.Count + ' 处静默吞错","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
