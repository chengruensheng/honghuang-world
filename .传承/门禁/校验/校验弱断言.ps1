# 校验弱断言.ps1 —— 检测仅含 len()/is_ok()/is_err() 的弱断言测试
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_weak_assertion.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "弱断言测试"
$weakTests = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -match "test|spec|#\[test\]" -and $_.FullName -notmatch "\\test|\\spec|\\target|\\工作区|\\.codeartsdoer" } | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $testBlocks = [regex]::Matches($content, "#\[test\]\s*fn\s+(\w+)[\s\S]*?\{([\s\S]*?)\n\}")
    foreach ($m in $testBlocks) {
        $body = $m.Groups[2].Value
        $剩余 = $body -replace 'assert_(?:eq|ne)!\([^)]*\.(?:len|is_ok|is_err)\(\)\)', ''
        $hasContentAssert = $剩余 -match 'assert_eq!|assert_ne!|assert!'
        $hasOnlyWeak = ($body -match "\.len\(\)|\.is_ok\(\)|\.is_err\(\)") -and -not $hasContentAssert
        if ($hasOnlyWeak) {
            $weakTests += "$($_.FullName): $($m.Groups[1].Value)"
        }
    }
}
if ($weakTests.Count -eq 0) {
    Write-Output '{"name":"弱断言测试","passed":true,"evidence":"未发现仅数量断言的弱测试","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"弱断言测试","passed":false,"evidence":"发现 ' + $weakTests.Count + ' 个弱断言测试","details":' + ($weakTests | ConvertTo-Json) + '}')
    exit 1
}
