# 校验命名.ps1 —— 检测命名一致性（禁用旧方法名）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_naming.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "命名一致性"
$oldMethodNames = @("按id", "按_id", "获取", "get_by_id", "fetch_")
$violations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "fn\s+(历史|按id|按_id|获取|get_by_id|fetch_)\w*\s*\(") {
            $violations += "$($_.FullName):$($i+1): 旧方法定义 $($Matches[1])（建议用 查询/全部/列表）"
        }
        if ($lines[$i] -match "(?<!\w)(历史|按id|获取)\s*\(" -and $lines[$i] -notmatch "fn\s+\w*历史") {
            if ($lines[$i] -match "\.历史\(|::历史\(|\.按id\(|::按id\(|\.获取\(|::获取\(") {
                $violations += "$($_.FullName):$($i+1): 旧方法调用（建议用 查询/全部/列表）"
            }
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"命名一致性","passed":true,"evidence":"未发现旧命名","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"命名一致性","passed":false,"evidence":"发现 ' + $violations.Count + ' 处旧命名","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
