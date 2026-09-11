# 校验规则空条件.ps1 —— 规则库 添加规则 是否对空条件返回错误
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_rule_empty_condition.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "规则库空条件API"
$found = $false
$hasValidation = $false
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -match "rule|规则" } | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    if ($content -match "fn\s+(?:添加规则|add_rule|新增规则)") {
        $found = $true
        if ($content -match "条件.*空|empty|is_empty|条件为空|返回错误|Err|条件不能为空") {
            $hasValidation = $true
        }
    }
}
if (-not $found) {
    Write-Output '{"name":"规则库空条件API","passed":null,"evidence":"未找到规则库添加规则方法","details":{}}'
    exit 0
} elseif ($hasValidation) {
    Write-Output '{"name":"规则库空条件API","passed":true,"evidence":"规则库添加规则对空条件有校验","details":{}}'
    exit 0
} else {
    Write-Output '{"name":"规则库空条件API","passed":false,"evidence":"规则库添加规则未对空条件返回错误","details":{}}'
    exit 1
}
