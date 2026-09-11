# 校验自动持久化.ps1 —— 检测状态变更方法是否缺持久化调用
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_auto_persist.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "自动持久化"
$violations = @()
$stateChangePatterns = @("新增|添加|创建|修改|更新|删除|移除|开启|关闭|完成|取消")
$persistPatterns = @("保存|持久化|save|persist|flush|写入|write")
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区|trait" } | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $fns = [regex]::Matches($content, "(?:pub\s+)?fn\s+(\w+)[\s\S]*?\{([\s\S]*?)\n\}")
    foreach ($m in $fns) {
        $fnName = $m.Groups[1].Value
        $body = $m.Groups[2].Value
        $hasStateChange = $false
        foreach ($p in $stateChangePatterns) {
            if ($body -match $p) { $hasStateChange = $true; break }
        }
        $hasPersist = $false
        foreach ($p in $persistPatterns) {
            if ($body -match $p) { $hasPersist = $true; break }
        }
        if ($hasStateChange -and -not $hasPersist -and $fnName -notmatch "new|default|init|from|to_") {
            $violations += "$($_.FullName): $fnName 状态变更但无持久化调用"
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"自动持久化","passed":true,"evidence":"状态变更方法均有持久化调用","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"自动持久化","passed":null,"evidence":"发现 ' + $violations.Count + ' 处状态变更可能缺持久化（需人工确认是否为内存-only操作）","details":' + ($violations | ConvertTo-Json) + '}')
    exit 0
}
