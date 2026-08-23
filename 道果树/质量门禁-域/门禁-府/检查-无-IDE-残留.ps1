# 检查-无-IDE-残留：根目录不得出现 IDE 工具本地配置目录与缓存文件。
# 依据：AGENTS.md 第 11 条「根目录严禁出现 IDE/工具本地配置目录（.arts/、.codeartsdoer/、.vscode/、.idea/ 等），
# 它们属于工具不属项目，已在 .gitignore 排除，但发现盘面仍存时直接删除」。
#
# 用法：pwsh -File 检查-无-IDE-残留.ps1 [-自动清理]
# 退出码：0 = 通过；1 = 发现残留（且未传 -自动清理 或 清理失败）
param([switch]$自动清理)

. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
Set-Location $根
Write-Host "工作区根：$根"

# IDE 工具本地配置目录与缓存文件清单
$禁止目录 = @('.arts', '.codeartsdoer', '.codegraph')
$禁止文件 = @('.merkle-snapshot.json')

$违规 = @()

foreach ($名 in $禁止目录) {
    $路径 = Join-Path $根 $名
    if (Test-Path $路径) {
        $违规 += "目录：$路径"
    }
}

foreach ($名 in $禁止文件) {
    $路径 = Join-Path $根 $名
    if (Test-Path $路径) {
        $违规 += "文件：$路径"
    }
}

# .gitignore 应屏蔽的 IDE 项对照（机械化：列出 .gitignore 中是否有相关条目）
$期望忽略 = @('.arts/', '.codeartsdoer/', '.merkle-snapshot.json')
$gitignore = Get-Content (Join-Path $根 '.gitignore') -Raw -Encoding UTF8
$缺失忽略 = @()
foreach ($项 in $期望忽略) {
    if ($gitignore -notmatch [regex]::Escape($项)) {
        $缺失忽略 += $项
    }
}

if ($违规.Count -gt 0) {
    Write-Host ""
    Write-Host "[发现残留] 根目录有以下 IDE 本地配置/缓存（应为项目外、不入库）："
    foreach ($项 in $违规) {
        Write-Host "  - $项"
    }
    if ($自动清理) {
        Write-Host ""
        Write-Host "[-自动清理] 尝试删除..."
        foreach ($项 in $违规) {
            $路径 = $项 -replace '^(目录|文件)：', ''
            try {
                if ($项.StartsWith('目录')) {
                    Remove-Item -Path $路径 -Recurse -Force -ErrorAction Stop
                } else {
                    Remove-Item -Path $路径 -Force -ErrorAction Stop
                }
                Write-Host "  [已清理] $路径"
            } catch {
                Write-Host "  [清理失败] $路径：$($_.Exception.Message)"
                exit 1
            }
        }
        Write-Host ""
        Write-Host "[通过] 自动清理完成"
        exit 0
    } else {
        Write-Host ""
        Write-Host "[拦截] 未传 -自动清理；请手动删除上述条目后重跑，或加 -自动清理 重试"
        exit 1
    }
}

if ($缺失忽略.Count -gt 0) {
    Write-Host ""
    Write-Host "[警告] .gitignore 缺少以下条目（建议补齐）："
    foreach ($项 in $缺失忽略) {
        Write-Host "  - $项"
    }
    Write-Host ""
    Write-Host "[失败] .gitignore 不完整，请补齐后重跑"
    exit 1
}

Write-Host "[通过] 根目录无 IDE 本地配置残留、.gitignore 屏蔽完整"
exit 0