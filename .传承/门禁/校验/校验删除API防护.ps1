# 校验删除API防护.ps1 —— 检测删除型 API 调用（取消最旧/淘汰最旧/...）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_delete_api.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "删除型API调用"
$patterns = @("取消最旧", "淘汰最旧", "回滚最旧", "废止最旧", "remove_oldest", "evict_oldest", "rollback_oldest", "cancel_oldest")
$violations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        foreach ($p in $patterns) {
            if ($lines[$i] -match [regex]::Escape($p) -and $lines[$i] -match "\.") {
                $violations += "$($_.FullName):$($i+1): 调用删除型API $p"
            }
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"删除型API调用","passed":true,"evidence":"未发现删除型API调用","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"删除型API调用","passed":false,"evidence":"发现 ' + $violations.Count + ' 处删除型API调用","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
