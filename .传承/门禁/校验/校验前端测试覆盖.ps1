# 校验前端测试覆盖.ps1 —— 运行 node:test 发现 *.test.mjs / *.spec.mjs
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_frontend_test.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "前端测试"

$搜索根 = @()
$乾坤 = Join-Path $ProjectRoot "乾坤"
$artifacts = Join-Path $ProjectRoot "artifacts/agent-workspace"
if (Test-Path $乾坤) { $搜索根 += $乾坤 }
if (Test-Path $artifacts) { $搜索根 += $artifacts }

$测试文件 = @()
foreach ($根 in $搜索根) {
    $测试文件 += Get-ChildItem -Path $根 -Recurse -File -Include "*.test.mjs", "*.spec.mjs" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "\\target\\|\\node_modules\\" }
}
if ($测试文件.Count -eq 0) {
    Write-Output ('{"name":"' + $name + '","passed":null,"evidence":"未发现前端测试文件（乾坤/*.test.mjs 或 artifacts/agent-workspace/*.test.mjs）","details":{}}')
    exit 0
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Output ('{"name":"' + $name + '","passed":null,"evidence":"未检测到 node，无法运行前端测试","details":{}}')
    exit 0
}

$输出 = ""
foreach ($根 in $搜索根) {
    $根测试 = Get-ChildItem -Path $根 -Recurse -File -Include "*.test.mjs", "*.spec.mjs" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "\\target\\|\\node_modules\\" }
    if ($根测试.Count -gt 0) {
        Push-Location $根
        $输出 += & node --test 2>&1 | Out-String
        Pop-Location
    }
}

$tests = 0; $pass = 0; $fail = 0
$t = [regex]::Match($输出, "tests\s+(\d+)")
if ($t.Success) { $tests = [int]$t.Groups[1].Value }
$p = [regex]::Match($输出, "pass\s+(\d+)")
if ($p.Success) { $pass = [int]$p.Groups[1].Value }
$f = [regex]::Match($输出, "fail\s+(\d+)")
if ($f.Success) { $fail = [int]$f.Groups[1].Value }

if ($tests -gt 0 -and $fail -eq 0) {
    Write-Output ('{"name":"' + $name + '","passed":true,"evidence":"前端测试 ' + $tests + ' 全绿（' + $pass + ' passed / ' + $fail + ' failed）","details":{"tests":' + $tests + ',"pass":' + $pass + ',"fail":' + $fail + '}}')
    exit 0
} elseif ($tests -eq 0) {
    Write-Output ('{"name":"' + $name + '","passed":null,"evidence":"未能解析前端测试结果，请手动运行 node --test 查看","details":{}}')
    exit 0
} else {
    Write-Output ('{"name":"' + $name + '","passed":false,"evidence":"前端测试存在失败（' + $fail + ' failed / ' + $tests + ' tests）","details":{"tests":' + $tests + ',"pass":' + $pass + ',"fail":' + $fail + '}}')
    exit 1
}
