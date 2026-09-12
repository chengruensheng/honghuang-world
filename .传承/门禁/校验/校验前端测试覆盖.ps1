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
# 呈现域源码一律用 `/xxx` 绝对路径 import（HTTP 托管根 = 域根，见 对外契约.toml 的 static_dir）。
# node 下不存在这个托管根：须经映射注册（resolve 钩子）把 `/xxx` 折回域根，
# 否则被测模块的依赖链整链解析失败——测试文件在、跑不起来，「覆盖」就是一句空话。
# 判据只用 Test-Path：映射注册本身就在乾坤内，文件在即乾坤在。
# 注意不可在此处比 `$根`——此刻它还不是循环变量，只是上方 foreach 的残留终值，
# 一旦 artifacts/agent-workspace 被重建（搜索根增至两项），残留值即为 artifacts 路径，
# 会导致 $映射URL 落空、乾坤下的测试整链解析失败，门禁报出假失败。
$映射注册 = Join-Path $乾坤 "观星呈现-域\呈现验证-府\映射装配-殿\钩子注册-阁\映射-模块-园\映射注册.mjs"
$映射URL = ""
if (Test-Path $映射注册) {
    $映射URL = "file:///" + ((Resolve-Path $映射注册).Path -replace '\\','/')
}
foreach ($根 in $搜索根) {
    $根测试 = Get-ChildItem -Path $根 -Recurse -File -Include "*.test.mjs", "*.spec.mjs" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "\\target\\|\\node_modules\\" }
    if ($根测试.Count -gt 0) {
        Push-Location $根
        # --test-isolation=process：node 24 默认 none，多测试文件同进程会共享模块实例与全局 document，
        # 一个文件的台面装配会顶掉另一个文件的 DOM，误报成断言失败。逐文件独立进程才是真结论。
        if (($根 -eq $乾坤) -and $映射URL) {
            $输出 += & node --test-isolation=process --import $映射URL --test 2>&1 | Out-String
        } else {
            $输出 += & node --test-isolation=process --test 2>&1 | Out-String
        }
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
