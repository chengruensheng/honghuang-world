# 验证-临时目录：AGENTS 第 9 条，临时文件夹只装临时产物。
# 范围：仅检查顶层 临时文件夹/（v1.0.0 后约定）。
# 注意：.上下文/.test-tmp/ 不在门禁范围——它是 cargo test 沙箱临时落点，由测试自身清理
# （当前通过 std::fs::remove_dir_all 在测试尾部清理，panic 时不清理是已知遗留，
# 后续 C 级任务「测试 Drop 自清临时目录」改完后，再把 .上下文/.test-tmp/ 加入门禁）。
. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
$路径 = Join-Path $根 '临时文件夹'
if (-not (Test-Path $路径)) {
    Write-Host '[通过] 临时文件夹 不存在'
    exit 0
}
$内容 = Get-ChildItem $路径 -Recurse -Force -ErrorAction SilentlyContinue
if ($null -eq $内容 -or $内容.Count -eq 0) {
    Write-Host '[通过] 临时文件夹 为空'
    exit 0
}
Write-Host '[警告] 临时文件夹 非空（' $内容.Count '项），任务结束应清空：'
$内容 | Select-Object -First 10 | ForEach-Object { Write-Host '  ' $_.Name }
if ($内容.Count -gt 10) { Write-Host '  ... 共 ' $内容.Count '项' }
exit 1
