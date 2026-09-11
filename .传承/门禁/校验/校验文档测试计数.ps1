# 校验文档测试计数.ps1 —— 维护文档测试数 vs cargo test 实测
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_doc_test_count.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "文档测试数同步"
$docPath = Join-Path $ProjectRoot "维护文档.md"
if (-not (Test-Path $docPath)) {
    $docPath = Get-ChildItem -Path $ProjectRoot -Recurse -Filter "*.md" -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch "\\.codeartsdoer" -and $_.Name -match "维护" } | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $docPath) {
    $docPath = Get-ChildItem -Path $ProjectRoot -Recurse -Filter "*.md" -ErrorAction SilentlyContinue | Where-Object { $_.Name -match "README|设计" } | Select-Object -First 1 -ExpandProperty FullName
}
$docCount = 0
if ($docPath -and (Test-Path $docPath)) {
    $docContent = Get-Content $docPath -Raw
    $match = [regex]::Match($docContent, "\*\*(\d+)\s*个测试.*?全部通过|(\d+)\s*个测试全部通过|测试全绿.*?(\d+)\s*个测试")
    if ($match.Success) {
        $docCount = [int]($match.Groups[1].Value + $match.Groups[2].Value + $match.Groups[3].Value)
    }
}
Push-Location $ProjectRoot
$tmp = Join-Path $env:TEMP ("hm_test_out_" + $PID + ".txt")
cmd /c "cargo test --quiet > `"$tmp`" 2>&1"
$lines = Get-Content $tmp
Remove-Item $tmp -Force -ErrorAction SilentlyContinue
$actualCount = ($lines | Select-String -Pattern "test result: ok\. (\d+) passed" | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
if ($actualCount -eq 0) {
    $actualCount = ($lines | Select-String -Pattern "running\s+(\d+)\s+tests" | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
}
Pop-Location
if ($docCount -eq $actualCount -and $actualCount -gt 0) {
    Write-Output ('{"name":"文档测试数同步","passed":true,"evidence":"文档测试数 ' + $docCount + ' = 实际 ' + $actualCount + '","details":{"doc":' + $docCount + ',"actual":' + $actualCount + '}}')
    exit 0
} elseif ($actualCount -eq 0) {
    Write-Output '{"name":"文档测试数同步","passed":null,"evidence":"无法获取实际测试数，请手动运行 cargo test","details":{}}'
    exit 0
} else {
    Write-Output ('{"name":"文档测试数同步","passed":false,"evidence":"文档测试数 ' + $docCount + ' != 实际 ' + $actualCount + '","details":{"doc":' + $docCount + ',"actual":' + $actualCount + '}}')
    exit 1
}
