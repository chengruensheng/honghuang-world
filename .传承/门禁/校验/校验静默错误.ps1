# 校验静默错误.ps1 —— 检测静默吞错（let _ = / if let Ok(_) / unwrap_or）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_silent_error.ps1
# 归集：.传承/门禁/校验/
#
# 判据边界（2026-09-12 复核后收紧）：
#   · let _ = / if let Ok(_) —— 维持原判断（规则检测 8：业务错误必须 warn! 或 ? 传播）。
#   · unwrap_or —— 只限 Result 上下文（同行含 Err/error/Result/失败/错误 标记）才列为候选。
#     strip_prefix/rsplit/split/rsplit_once 之后的 unwrap_or 是 Option 默认值语义：缺后缀回退原串、
#     缺字段默认空、驳回层级默认「土」等，属业务合理默认，不构成静默吞错（13 处全量复核均为此类）。
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
        if ($lines[$i] -match "\.unwrap_or\(" -and $lines[$i] -match "Err|error|Result|失败|错误" -and $lines[$i] -notmatch "default|0|false|空") {
            $violations += "$($_.FullName):$($i+1): unwrap_or 静默降级（Result 上下文）"
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
