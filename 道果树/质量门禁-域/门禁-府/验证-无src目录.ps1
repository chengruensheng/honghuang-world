# 验证-无src目录：AGENTS 第 10 条，根目录或子目录严禁 src/ tests/ scripts/ 平铺
. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
$违规 = @()
Get-ChildItem $根 -Recurse -Directory | Where-Object {
    $_.FullName -notmatch '\\道果树\\|\\\.上下文\\|\\临时文件夹\\|\\\.git\\'
} | ForEach-Object {
    $名 = $_.Name
    if ($名 -eq 'src' -or $名 -eq 'tests' -or $名 -eq 'scripts') {
        $违规 += $_.FullName
    }
}
if ($违规.Count -gt 0) {
    Write-Host "[失败] 发现平铺源码目录（AGENTS 第 10 条）："
    $违规 | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "[通过] 无 src/tests/scripts/ 平铺目录"
exit 0