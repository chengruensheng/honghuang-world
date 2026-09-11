# 校验空条件.ps1 —— 检查「添加规则」的条件参数是否为空 vec![]
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_empty_condition.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "空条件规则"
$violations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "添加规则|add_rule|新增规则" -and $lines[$i] -match "vec!\[\]") {
            $violations += "$($_.FullName):$($i+1): 空条件规则"
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"空条件规则","passed":true,"evidence":"未发现空条件规则","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"空条件规则","passed":false,"evidence":"发现 ' + $violations.Count + ' 处空条件规则","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
