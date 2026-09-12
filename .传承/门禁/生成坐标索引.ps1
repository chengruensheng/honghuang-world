# 生成坐标索引.ps1 —— 坐标索引通用引擎（零项目名词）
# 定位：知识图谱第 1 阶段（坐标层）。
# 机制：命名后缀驱动的多层分类树解析 —— 根名单、层数与后缀、排除名单全部来自定义文件。
# 定义：默认读 .传承/图谱/知识图谱/坐标定义.json（可用 -定义文件 覆盖）
# 产出：<定义.输出.目录>/<JSON文件名>（机器消费）+ <MD文件名>（人读摘要）
# 退出码：0 = 生成成功；非零 = 失败
param(
    [string]$定义文件
)

$ErrorActionPreference = 'Stop'

# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrEmpty($定义文件)) {
    $定义文件 = Join-Path $根 '.传承\图谱\知识图谱\坐标定义.json'
}
if (-not (Test-Path -LiteralPath $定义文件)) {
    Write-Host "[缺] 坐标定义文件未找到：$定义文件" -ForegroundColor Red
    exit 1
}
$定义 = Get-Content -LiteralPath $定义文件 -Raw -Encoding UTF8 | ConvertFrom-Json

# ---- 从定义构建运行时结构（以下不含任何项目名词） ----
$项目名 = if ($定义.项目名) { [string]$定义.项目名 } else { (Get-Item -LiteralPath $根).Name }
$索引名 = if ($定义.索引名) { [string]$定义.索引名 } else { '坐标索引' }

$根名单 = @($定义.根 | ForEach-Object { [string]$_.名 })
$根属性 = @{}
foreach ($r in $定义.根) {
    $根属性[[string]$r.名] = @{
        '判层'   = if ($null -ne $r.判层) { [bool]$r.判层 } else { $true }
        '递归'   = if ($null -ne $r.递归) { [bool]$r.递归 } else { $true }
        '可重建' = if ($null -ne $r.可重建) { [bool]$r.可重建 } else { $false }
    }
}

$层序 = @($定义.层 | ForEach-Object { [string]$_.名 })
$层后缀 = @{}
foreach ($l in $定义.层) { $层后缀[[string]$l.名] = [string]$l.后缀 }
if ($层序.Count -eq 0) {
    Write-Host '[缺] 坐标定义中「层」为空' -ForegroundColor Red
    exit 1
}

$排除名 = [System.Collections.Generic.List[string]]::new()
foreach ($n in $定义.排除名) { $排除名.Add([string]$n) }

$输出目录 = Join-Path $根 ([string]$定义.输出.目录 -replace '/', '\')
$JSON名 = [string]$定义.输出.JSON文件名
$MD名 = [string]$定义.输出.MD文件名
# 排除产出文件自身：避免自指导致重跑数字漂移，保证幂等
$排除名.Add($JSON名)
$排除名.Add($MD名)

$节点 = [System.Collections.Generic.List[object]]::new()
$异常 = [System.Collections.Generic.List[object]]::new()

function 取层级([string]$名) {
    foreach ($层 in $层序) {
        if ($名.EndsWith($层后缀[$层])) { return $层 }
    }
    return $null
}

function 扫描([string]$物理路径, [string]$相对路径, $坐标, [bool]$解析层级) {
    $子项 = @(Get-ChildItem -LiteralPath $物理路径 -Force -ErrorAction SilentlyContinue |
            Where-Object { $排除名 -notcontains $_.Name } |
            Sort-Object @{Expression = { $_.PSIsContainer }; Descending = $true }, @{Expression = { $_.Name } })

    foreach ($项 in $子项) {
        $相对 = if ([string]::IsNullOrEmpty($相对路径)) { $项.Name } else { "$相对路径/$($项.Name)" }

        if ($项.PSIsContainer) {
            $层 = if ($解析层级) { 取层级 $项.Name } else { $null }
            $子坐标 = [ordered]@{}
            foreach ($k in $坐标.Keys) { $子坐标[$k] = $坐标[$k] }
            if ($层) { $子坐标[$层] = $项.Name }

            # 异常判定：仅对参与坐标解析的树
            if ($解析层级) {
                $已有层数 = @($坐标.Keys | Where-Object { $_ -ne '根' }).Count
                if ($已有层数 -eq 0 -and $层 -ne $层序[0]) {
                    $异常.Add([ordered]@{ '路径' = $相对; '类型' = '目录'; '原因' = "根下第一层非$($层序[0])（期望 *$($层后缀[$层序[0]])）" })
                }
                elseif ($层 -and $已有层数 -lt $层序.Count -and $层 -ne $层序[$已有层数]) {
                    $异常.Add([ordered]@{ '路径' = $相对; '类型' = '目录'; '原因' = "层级跳跃（期望 $($层序[$已有层数])，实为 $层）" })
                }
                elseif (-not $层) {
                    $异常.Add([ordered]@{ '路径' = $相对; '类型' = '目录'; '原因' = '未归类目录（无层后缀）' })
                }
            }

            $节点.Add([ordered]@{
                    '路径' = $相对
                    '类型' = '目录'
                    '层级' = $层
                    '坐标' = $子坐标
                    '长度' = $null
                    '更新' = $项.LastWriteTime.ToString('yyyy-MM-dd')
                })
            扫描 -物理路径 $项.FullName -相对路径 $相对 -坐标 $子坐标 -解析层级 $解析层级
        }
        else {
            $节点.Add([ordered]@{
                    '路径' = $相对
                    '类型' = '文件'
                    '层级' = $null
                    '坐标' = $坐标
                    '长度' = $项.Length
                    '更新' = $项.LastWriteTime.ToString('yyyy-MM-dd')
                })
        }
    }
}

# 一、根（按定义顺序）
foreach ($根名 in $根名单) {
    $物理 = Join-Path $根 $根名
    if (-not (Test-Path -LiteralPath $物理)) { continue }

    $属性 = $根属性[$根名]
    $初始坐标 = [ordered]@{ '根' = $根名 }
    $项 = Get-Item -LiteralPath $物理

    if (-not $属性.递归) {
        # 可整体重建区：只记根节点 + 体积画像，不逐条收录（其内容无独立演化价值）
        $内容 = @(Get-ChildItem -LiteralPath $物理 -Force -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $排除名 -notcontains $_.Name })
        $内容文件 = @($内容 | Where-Object { -not $_.PSIsContainer })
        $体积 = ($内容文件 | Measure-Object -Property Length -Sum).Sum
        if ($null -eq $体积) { $体积 = 0 }
        $节点.Add([ordered]@{
                '路径'       = $根名
                '类型'       = '目录'
                '层级'       = $null
                '坐标'       = $初始坐标
                '长度'       = $null
                '更新'       = $项.LastWriteTime.ToString('yyyy-MM-dd')
                '可重建'     = $属性.可重建
                '内容目录数' = @($内容 | Where-Object { $_.PSIsContainer }).Count
                '内容文件数' = $内容文件.Count
                '内容体积'   = $体积
            })
        continue
    }

    $节点.Add([ordered]@{
            '路径' = $根名
            '类型' = '目录'
            '层级' = $null
            '坐标' = $初始坐标
            '长度' = $null
            '更新' = $项.LastWriteTime.ToString('yyyy-MM-dd')
        })
    扫描 -物理路径 $物理 -相对路径 $根名 -坐标 $初始坐标 -解析层级 $属性.判层
}

# 二、根级散落文件
$根级文件 = @(Get-ChildItem -LiteralPath $根 -Force -File -ErrorAction SilentlyContinue |
        Where-Object { $排除名 -notcontains $_.Name } |
        Sort-Object Name)
foreach ($f in $根级文件) {
    $节点.Add([ordered]@{
            '路径' = $f.Name
            '类型' = '文件'
            '层级' = $null
            '坐标' = [ordered]@{ '根' = '(仓库根)' }
            '长度' = $f.Length
            '更新' = $f.LastWriteTime.ToString('yyyy-MM-dd')
        })
}

# 三、统计
$目录数 = @($节点 | Where-Object { $_.类型 -eq '目录' }).Count
$文件数 = @($节点 | Where-Object { $_.类型 -eq '文件' }).Count

$按层 = [ordered]@{}
foreach ($层 in $层序) { $按层[$层] = 0 }

$按根 = [ordered]@{}
foreach ($r in ($根名单 + @('(仓库根)'))) {
    $行 = [ordered]@{ '目录' = 0; '文件' = 0 }
    foreach ($层 in $层序) { $行[$层] = 0 }
    $按根[$r] = $行
}

foreach ($n in $节点) {
    $r = [string]$n.坐标['根']
    if (-not $按根.Contains($r)) { continue }
    if ($n.类型 -eq '目录') {
        $按根[$r]['目录']++
        if ($n.层级) {
            $按根[$r][$n.层级]++
            $按层[$n.层级]++
        }
    }
    else {
        $按根[$r]['文件']++
    }
}

# 四、写 JSON（机器消费）
if (-not (Test-Path -LiteralPath $输出目录)) { New-Item -ItemType Directory -Path $输出目录 -Force | Out-Null }
$时间 = Get-Date -Format 'yyyy-MM-dd HH:mm'

$结果 = [ordered]@{
    '元信息' = [ordered]@{
        '图谱'     = $索引名
        '版本'     = [string]$定义.版本
        '生成时间' = $时间
        '仓库根'   = $根
        '定义文件' = $定义文件
        '生成器'   = '.传承/门禁/生成坐标索引.ps1'
        '说明'     = '把仓库路径机械解析为多层语义坐标；坐标由路径推导，不手填。'
    }
    '统计'   = [ordered]@{
        '节点总数' = $节点.Count
        '目录数'   = $目录数
        '文件数'   = $文件数
        '异常数'   = $异常.Count
        '按层'     = $按层
        '按根'     = $按根
    }
    '节点'   = $节点
    '异常'   = $异常
}
$json路径 = Join-Path $输出目录 $JSON名
[System.IO.File]::WriteAllText($json路径, ($结果 | ConvertTo-Json -Depth 12), (New-Object System.Text.UTF8Encoding($false)))

# 五、写 Markdown（人读摘要）
$md = [System.Collections.Generic.List[string]]::new()
$md.Add('# ' + $项目名 + ' · 知识图谱 · ' + $索引名)
$md.Add('')
$md.Add('> **定位**：知识图谱第 1 阶段（坐标层）——把仓库路径机械解析为多层语义坐标。')
$md.Add('> **原理**：坐标由路径推导（层后缀由 `坐标定义.json` 声明），不手填、零维护。')
$md.Add('> **定义**：`坐标定义.json`（项目特化，数据）；**引擎**：`.传承/门禁/生成坐标索引.ps1`（通用，零项目名词）。')
$md.Add('> **数据本体**：`' + $JSON名 + '`（机器消费，本文件只是摘要）')
$md.Add('> **生成时间**：' + $时间)
$md.Add('')
$md.Add('---')
$md.Add('')
$md.Add('## 一、总览')
$md.Add('')
$md.Add('| 指标 | 数值 |')
$md.Add('|---|---|')
$md.Add('| 节点总数 | ' + $节点.Count + ' |')
$md.Add('| 目录数 | ' + $目录数 + ' |')
$md.Add('| 文件数 | ' + $文件数 + ' |')
$md.Add('| 异常数 | ' + $异常.Count + ' |')
$md.Add('')
$md.Add('## 二、按根分布')
$md.Add('')
$表头 = '| 根 |'
$分隔 = '|---|'
foreach ($层 in $层序) { $表头 += " $层 |"; $分隔 += '---|' }
$表头 += ' 目录 | 文件 |'
$分隔 += '---|---|'
$md.Add($表头)
$md.Add($分隔)
foreach ($r in ($根名单 + @('(仓库根)'))) {
    $v = $按根[$r]
    $行 = "| $r |"
    foreach ($层 in $层序) { $行 += " $($v[$层]) |" }
    $行 += " $($v['目录']) | $($v['文件']) |"
    $md.Add($行)
}
$md.Add('')
$可重建根 = @($定义.根 | Where-Object { $_.可重建 -eq $true } | ForEach-Object { [string]$_.名 })
if ($可重建根.Count -gt 0) {
    $md.Add('> **注**：' + ($可重建根 -join '、') + ' 为可整体重建的临时物集中地，只记根节点与体积画像，不逐条收录；体积见 JSON 中 `可重建: true` 的节点。')
    $md.Add('')
}
$md.Add('## 三、层级统计')
$md.Add('')
$md.Add('| 层 | 数量 |')
$md.Add('|---|---|')
foreach ($层 in $层序) { $md.Add("| $层 | $($按层[$层]) |") }
$md.Add('')
$md.Add('## 四、异常清单')
$md.Add('')
if ($异常.Count -eq 0) {
    $md.Add('（无）')
}
else {
    $md.Add('| 路径 | 类型 | 原因 |')
    $md.Add('|---|---|---|')
    foreach ($e in $异常) { $md.Add("| $($e['路径']) | $($e['类型']) | $($e['原因']) |") }
}
$md.Add('')
$md.Add('---')
$md.Add('')
$md.Add('## 五、后续阶段')
$md.Add('')
$md.Add('| 阶段 | 内容 | 状态 |')
$md.Add('|---|---|---|')
$md.Add('| 1 | 坐标索引（本文件）：路径 → 多层语义坐标 | **已完成** |')
$md.Add('| 2 | 符号索引：crate / 模块 / 函数 / trait / impl / use / call | **已完成** |')
$md.Add('| 3 | 声明索引：契约 / 规则条文 / 文档引用 | 待做 |')
$md.Add('| 4 | 合并与诊断：dead / orphan / drift / broken_contract，接入门禁 | 进行中（探针已出：命名/坐标合规） |')

$md路径 = Join-Path $输出目录 $MD名
[System.IO.File]::WriteAllText($md路径, (($md -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))

"坐标索引已生成：节点 $($节点.Count)（目录 $目录数 / 文件 $文件数）/ 异常 $($异常.Count)"
"→ $json路径（$((Get-Item -LiteralPath $json路径).Length) 字节）"
"→ $md路径"
exit 0
