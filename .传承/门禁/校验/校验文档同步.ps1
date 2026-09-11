# 校验文档同步.ps1 —— 维护文档声明测试数 vs cargo test 实测数
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_doc_sync.ps1
# 归集：.传承/门禁/校验/
param(
    [string]$ProjectRoot = ".",
    [switch]$Write
)
$name = "文档数自动比对"
$root = $ProjectRoot

$维护Doc = Join-Path $root "传承殿/维护文档.md"
if (-not (Test-Path $维护Doc)) {
    $维护Doc = Get-ChildItem -Path $root -Recurse -Filter "*.md" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "\\.codeartsdoer" -and $_.Name -match "维护" } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $维护Doc) {
    Write-Output '{"name":"文档数自动比对","passed":null,"evidence":"未找到维护文档.md，无法比对声明测试数","details":{}}'
    exit 0
}

$docCount = 0
$文档数字 = ""
if (Test-Path $维护Doc) {
    $内容 = Get-Content $维护Doc -Raw
    $全局 = [regex]::Matches($内容, "全量\s*(\d+)\s*全绿|(\d+)\s*个测试全部通过")
    if ($全局.Count -gt 0) {
        $最后 = $全局[$全局.Count - 1]
        $文档数字 = $最后.Groups[1].Value + $最后.Groups[2].Value
        if ($文档数字) { $docCount = [int]$文档数字 }
    }
}

Push-Location $root
$tmp = Join-Path $env:TEMP ("hm_test_out_" + $PID + ".txt")
cmd /c "cargo test --quiet > `"$tmp`" 2>&1"
$lines = Get-Content $tmp
Remove-Item $tmp -Force -ErrorAction SilentlyContinue
$actualCount = ($lines | Select-String -Pattern "test result: ok\. (\d+) passed" | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
if ($actualCount -eq 0) {
    $actualCount = ($lines | Select-String -Pattern "running\s+(\d+)\s+tests" | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
}
Pop-Location

if ($Write -and $actualCount -gt 0 -and $docCount -ne $actualCount) {
    $内容 = Get-Content $维护Doc -Raw
    $全局 = [regex]::Matches($内容, "全量\s*\d+\s*全绿")
    if ($全局.Count -gt 0) {
        $最后 = $全局[$全局.Count - 1]
        $新内容 = $内容.Substring(0, $最后.Index) + ("全量 {0} 全绿" -f $actualCount) + $内容.Substring($最后.Index + $最后.Length)
        Set-Content -Path $维护Doc -Value $新内容 -Encoding UTF8
        $docCount = $actualCount
    }
}

if ($docCount -gt 0 -and $actualCount -gt 0) {
    if ($docCount -eq $actualCount) {
        Write-Output ('{"name":"文档数自动比对","passed":true,"evidence":"文档最新声明 ' + $docCount + ' = 实测 ' + $actualCount + '","details":{"doc":' + $docCount + ',"actual":' + $actualCount + '}}')
        exit 0
    } else {
        Write-Output ('{"name":"文档数自动比对","passed":false,"evidence":"文档最新声明 ' + $docCount + ' != 实测 ' + $actualCount + '，文档数字须同步（可带 -Write 自动回填）","details":{"doc":' + $docCount + ',"actual":' + $actualCount + '}}')
        exit 1
    }
} elseif ($actualCount -eq 0) {
    Write-Output '{"name":"文档数自动比对","passed":null,"evidence":"无法获取实际测试数，请手动运行 cargo test","details":{}}'
    exit 0
} else {
    Write-Output '{"name":"文档数自动比对","passed":null,"evidence":"维护文档未找到测试数声明，无法比对（建议用 -Write 生成）","details":{}}'
    exit 0
}
