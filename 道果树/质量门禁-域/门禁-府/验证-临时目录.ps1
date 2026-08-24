# 验证-临时目录：AGENTS 第 9 条，临时文件夹只装临时产物
. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
$临时 = Join-Path $根 '临时文件夹'
if (-not (Test-Path $临时)) {
    Write-Host "[通过] 临时文件夹不存在"
    exit 0
}
$内容 = Get-ChildItem $临时 -Recurse -Force
if ($内容.Count -eq 0) {
    Write-Host "[通过] 临时文件夹为空"
    exit 0
}
Write-Host "[警告] 临时文件夹非空（$($内容.Count) 项），任务结束应清空："
$内容 | Select-Object -First 10 | ForEach-Object { Write-Host "  $($_.Name)" }
if ($内容.Count -gt 10) { Write-Host "  ... 共 $($内容.Count) 项" }
exit 0