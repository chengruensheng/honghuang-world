# 验证-无空目录：检查源码区无空目录（AGENTS 第 10 条）
. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
$空目录 = @()
Get-ChildItem $根 -Recurse -Directory | Where-Object {
    $_.FullName -notmatch '\\道果树\\|\\\.上下文\\|\\临时文件夹\\|临时文件夹$|\\\.git\\'
} | ForEach-Object {
    $子项 = Get-ChildItem $_.FullName -Force
    if ($子项.Count -eq 0) {
        $空目录 += $_.FullName
    }
}
if ($空目录.Count -gt 0) {
    Write-Host "[失败] 发现空目录："
    $空目录 | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "[通过] 无空目录"
exit 0