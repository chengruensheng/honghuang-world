# 生成「洪荒·世界」项目目录树 → 工作区/项目目录树.md
# 范围：工程主体（八根白名单目录 + 根级文件）；排除 target/.git/工具目录/道果/工作区
$ErrorActionPreference = 'Stop'

# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$白名单目录 = @('乾坤', '太初', '量劫', '道韵', '混沌', '证道', '鸿蒙', '传承殿', 'rules')
$排除名 = @('target', '.git', '.codeartsdoer', '.codebuddy', '.trae', '.workbuddy', 'node_modules', '__pycache__')
$树行 = [System.Collections.Generic.List[string]]::new()
$script:目录数 = 0
$script:文件数 = 0

function 走($目录, $前缀) {
    $子项 = @(Get-ChildItem -LiteralPath $目录 -Force -ErrorAction SilentlyContinue |
        Where-Object { $排除名 -notcontains $_.Name } |
        Sort-Object @{Expression = { $_.PSIsContainer }; Descending = $true }, @{Expression = { $_.Name } })
    for ($i = 0; $i -lt $子项.Count; $i++) {
        $项 = $子项[$i]
        $末 = ($i -eq $子项.Count - 1)
        $枝 = if ($末) { '└── ' } else { '├── ' }
        if ($项.PSIsContainer) {
            $script:目录数++
            $树行.Add($前缀 + $枝 + $项.Name + '/')
            $子前缀 = $前缀 + $(if ($末) { '    ' } else { '│   ' })
            走 -目录 $项.FullName -前缀 $子前缀
        }
        else {
            $script:文件数++
            $树行.Add($前缀 + $枝 + $项.Name)
        }
    }
}

$树行.Add('洪荒 - 世界/')
$script:目录数++  # 计入项目根自身，使统计口径完整
$顶层 = @(Get-ChildItem -LiteralPath $根 -Force -ErrorAction SilentlyContinue |
    Where-Object { ($_.PSIsContainer -and ($白名单目录 -contains $_.Name)) -or (-not $_.PSIsContainer) } |
    Sort-Object @{Expression = { $_.PSIsContainer }; Descending = $true }, @{Expression = { $_.Name } })
for ($i = 0; $i -lt $顶层.Count; $i++) {
    $项 = $顶层[$i]
    $末 = ($i -eq $顶层.Count - 1)
    $枝 = if ($末) { '└── ' } else { '├── ' }
    if ($项.PSIsContainer) {
        $script:目录数++
        $树行.Add($枝 + $项.Name + '/')
        走 -目录 $项.FullName -前缀 $(if ($末) { '    ' } else { '│   ' })
    }
    else {
        $script:文件数++
        $树行.Add($枝 + $项.Name)
    }
}

$时间 = Get-Date -Format 'yyyy-MM-dd HH:mm'
$头部行 = @(
    '# 洪荒·世界 · 项目目录树',
    '',
    '> **范围**：工程主体——乾坤 / 太初 / 量劫 / 道韵 / 混沌 / 证道 / 鸿蒙 七根 + 传承殿 + rules，以及根级文件。',
    '> **排除**：构建产物 `target`、版本控制 `.git`、工具目录（`.codeartsdoer` / `.trae` / `.codebuddy` / `.workbuddy`）、运行时数据 `道果`、开发产物区 `工作区`。',
    '> **粒度**：完整到文件（同级「目录在前、文件在后」，各自按名称排序）。',
    "> **生成时间**：$时间",
    "> **统计**：目录 $script:目录数 个（含项目根自身）/ 文件 $script:文件数 个。",
    '',
    '---',
    '',
    '```text'
)
$内容 = ($头部行 -join "`n") + "`n" + ($树行 -join "`n") + "`n" + '```' + "`n"
$目标 = Join-Path $根 '工作区\项目目录树.md'
[System.IO.File]::WriteAllText($目标, $内容, (New-Object System.Text.UTF8Encoding($false)))
"目录 $script:目录数 个 / 文件 $script:文件数 个；已写入 $目标（$((Get-Item -LiteralPath $目标).Length) 字节）"
