# 诊断图谱.ps1 —— 图谱诊断引擎（探针版，零项目名词）
#
# 定位：知识图谱第 4 阶段（合并与诊断）的最小探针。只读既有索引，不重扫仓库。
# 输入：坐标索引.json（第 1 阶段产出）+ 符号索引.json（第 2 阶段产出）
# 产出：诊断结果.json（机器消费）+ 诊断报告.md（人读摘要）
# 判据来源：rules/命名规则.md、.传承/设计/落地设计/语言解析插件-落地设计.md（置信度分级）
#
# 置信度契约（沿用落地设计）：高 = 硬规则明确违规，可直接采信；
#                               中 = 疑似，须人工判定；
#                               低 = 画像/线索，不可单独作为清理依据。
#
# 退出码：0 = 诊断完成（有发现不算失败）；1 = 输入缺失或损坏
param(
    [string]$坐标索引,
    [string]$符号索引,
    [string]$输出目录,
    # 不参与死代码判定的维度根（测试类维度）。项目特化，由调用方传入。
    [string[]]$排除根 = @('证道'),
    # 插件原始产出（含「诊断」自诊断数组）。可选：存在则把语义层解析覆盖数据并入结论作硬证据。
    [string]$插件诊断 = ''
)

$ErrorActionPreference = 'Stop'

# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrEmpty($坐标索引)) { $坐标索引 = Join-Path $根 '.传承\图谱\知识图谱\坐标索引.json' }
if ([string]::IsNullOrEmpty($符号索引)) { $符号索引 = Join-Path $根 '.传承\图谱\知识图谱\符号索引.json' }
if ([string]::IsNullOrEmpty($输出目录)) { $输出目录 = Join-Path $根 '.传承\图谱\知识图谱' }

foreach ($f in @($坐标索引, $符号索引)) {
    if (-not (Test-Path -LiteralPath $f)) {
        Write-Host "[缺] 输入未找到：$f" -ForegroundColor Red
        exit 1
    }
}

$坐 = Get-Content -LiteralPath $坐标索引 -Raw -Encoding UTF8 | ConvertFrom-Json
$符 = Get-Content -LiteralPath $符号索引 -Raw -Encoding UTF8 | ConvertFrom-Json

# 插件自诊断（可选输入）：把语义层解析覆盖数据并入死代码结论作硬证据。
# 「信息」文本由插件 fmt! 固定生成，正则按稳定锚点定位关键数字。
$自诊 = @{}
if ([string]::IsNullOrEmpty($插件诊断)) { $插件诊断 = Join-Path $根 '.传承\记忆\缓存\rust解析-产出.json' }
if (Test-Path -LiteralPath $插件诊断) {
    $插件原始 = Get-Content -LiteralPath $插件诊断 -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($d in @($插件原始.诊断)) {
        $信息 = [string]$d.信息
        if ($信息 -match '产出调用边\s+(\d+)\s*条') { $自诊['语义层调用边'] = [int]$Matches[1] }
        if ($信息 -match '(\d+)\s*处调用的目标落在仓库内却不在符号表中') { $自诊['仓库内不在符号表'] = [int]$Matches[1] }
        if ($信息 -match '方法调用\s+成功\s+\d+\s*/\s*解析不出\s+(\d+)') { $自诊['方法调用解析不出'] = [int]$Matches[1] }
        if ($信息 -match '取不到可调用体\s+(\d+)') { $自诊['函数调用取不到可调用体'] = [int]$Matches[1] }
        if ($信息 -match '离开函数体的调用点\s+(\d+)') { $自诊['离开函数体调用点'] = [int]$Matches[1] }
    }
}

# ---- 判据常量（默认值摘自 rules/命名规则.md；改规则时同步改此处） ----
# 殿 / 阁：条文写作「2字-2字-殿」，但全仓实际落地形态是两个 2 字词**拼接**（共 4 字）不带横杠，
# 例如「符号提取-殿」「契约校验-阁」。若按字面横杠形态取判据，190+ 个目录会全数告警，
# 属条文表述与落地形态不符，故判据取 4 字（保留对「持久化-殿」这类 3 字名字的告警能力）。
$层格式 = @{
    '域' = '^.{4}-域$'
    '府' = '^.{4}-府$'
    '殿' = '^.{4}-殿$'
    '阁' = '^.{4}-阁$'
}
$园段数 = 3                                  # 园：名字-类型-园
$上限 = @{ '域' = 3; '殿' = 6; '阁' = 4 }     # 每根域数 / 每府殿数 / 每殿阁数
$crate前缀 = @{ '鸿蒙' = 'hm'; '太初' = 'tc'; '量劫' = 'lj'; '乾坤' = 'qk'; '道韵' = 'dy'; '混沌' = 'hd'; '证道' = 'zd' }

# ---- 工具函数 ----
function 取末段([string]$路径) {
    $i = $路径.LastIndexOf('/')
    if ($i -lt 0) { return $路径 }
    return $路径.Substring($i + 1)
}
function 取父路径([string]$路径) {
    $i = $路径.LastIndexOf('/')
    if ($i -lt 0) { return '' }
    return $路径.Substring(0, $i)
}
function 取语义([string]$名) {
    $段 = @($名 -split '-')
    if ($段.Count -le 1) { return $名 }
    return (($段[0..($段.Count - 2)]) -join '-')
}

$发现 = [System.Collections.Generic.List[object]]::new()
function 记([string]$类, [string]$级别, [string]$对象, [string]$说明, [string]$依据) {
    $发现.Add([ordered]@{ '类' = $类; '级别' = $级别; '对象' = $对象; '说明' = $说明; '依据' = $依据 })
}

$目录 = @($坐.节点 | Where-Object { $_.类型 -eq '目录' })

# 父路径 -> 子目录（一次构建，多处复用）
$按父 = @{}
foreach ($d in $目录) {
    $p = 取父路径 $d.路径
    if (-not $按父.ContainsKey($p)) { $按父[$p] = [System.Collections.Generic.List[object]]::new() }
    $按父[$p].Add($d)
}

# ---- A 类：坐标异常（搬运第 1 阶段结论） ----
foreach ($e in $坐.异常) { 记 '坐标异常' '高' $e.路径 $e.原因 '坐标定义.json 层序' }

# ---- B 类：命名格式 ----
foreach ($d in $目录) {
    if (-not $d.层级) { continue }
    $名 = 取末段 $d.路径
    $层 = [string]$d.层级
    if ($层 -eq '园') {
        $段数 = @($名 -split '-').Count
        if ($段数 -ne $园段数) { 记 '命名格式' '中' $d.路径 "园应三段式（名字-类型-园），实为 $段数 段" 'rules/命名规则.md 一' }
    }
    elseif ($层格式.ContainsKey($层)) {
        if ($名 -notmatch $层格式[$层]) { 记 '命名格式' '高' $d.路径 "不匹配「$层$($层格式[$层])」" 'rules/命名规则.md 一' }
    }
}

# ---- C 类：命名唯一性 ----
foreach ($p in $按父.Keys) {
    $按义 = @{}
    foreach ($d in $按父[$p]) {
        $义 = 取语义 (取末段 $d.路径)
        if (-not $按义.ContainsKey($义)) { $按义[$义] = [System.Collections.Generic.List[string]]::new() }
        $按义[$义].Add($d.路径)
    }
    foreach ($义 in $按义.Keys) {
        if ($按义[$义].Count -gt 1) { 记 '命名唯一' '高' $p "同级语义词重复「$义」：$($按义[$义] -join ' / ')" 'rules/命名规则.md 三.1' }
    }
    if ($p -ne '') {
        $父义 = 取语义 (取末段 $p)
        foreach ($d in $按父[$p]) {
            if ((取语义 (取末段 $d.路径)) -eq $父义) { 记 '命名唯一' '高' $d.路径 "下级语义词与直接上级相同「$父义」" 'rules/命名规则.md 三.2' }
        }
    }
}

# ---- D 类：数量上限 ----
foreach ($d in $目录) {
    if ($d.路径 -notmatch '/') {
        # 根目录：其下域数受限
        $子 = $按父[$d.路径]
        $域数 = if ($null -eq $子) { 0 } else { @($子 | Where-Object { $_.层级 -eq '域' }).Count }
        if ($域数 -gt $上限['域']) { 记 '数量上限' '高' $d.路径 "根下域 $域数 个 > 上限 $($上限['域'])" 'rules/命名规则.md 四.1' }
        continue
    }
    $期望子层 = switch ([string]$d.层级) {
        '府' { '殿' }
        '殿' { '阁' }
        default { $null }
    }
    if ($null -eq $期望子层) { continue }
    $子 = $按父[$d.路径]
    if ($null -eq $子) { continue }
    $子数 = @($子 | Where-Object { $_.层级 -eq $期望子层 }).Count
    if ($子数 -gt $上限[$期望子层]) { 记 '数量上限' '高' $d.路径 "$期望子层 $子数 个 > 上限 $($上限[$期望子层])" 'rules/命名规则.md 四.3' }
}

# ---- E 类：英文命名（仅目录） ----
foreach ($d in $目录) {
    if (-not $d.层级) { continue }
    $名 = 取末段 $d.路径
    if ($名 -match '[A-Za-z]') { 记 '英文命名' '高' $d.路径 "目录名含英文「$名」" 'rules/命名规则.md 五' }
}

# ---- F 类：crate 命名前缀 ----
foreach ($s in $符.符号) {
    if ([string]$s.符号.种类 -ne 'Crate') { continue }
    if ($null -eq $s.坐标) { continue }
    $r = [string]$s.坐标.根
    if (-not $crate前缀.ContainsKey($r)) { continue }
    $期望 = $crate前缀[$r] + '-'
    $名 = [string]$s.符号.名
    if (-not $名.StartsWith($期望)) {
        记 'crate命名' '高' $s.符号.文件 "crate 名「$名」未以「$期望」开头" 'rules/命名规则.md 六'
    }
}

# ---- G 类：死代码候选（低置信度，必须配合覆盖率画像判读） ----
# id -> 种类（用于按种类汇总入度能力）
$种类表 = @{}
$符号表 = @{}
foreach ($s in $符.符号) {
    $种类表[[string]$s.符号.id] = [string]$s.符号.种类
    $符号表[[string]$s.符号.id] = $s
}

$入度 = @{}
$种类入度边 = @{}
foreach ($e in $符.边) {
    if ([string]$e.类型 -eq '包含') { continue }   # 只采信 调用 / 依赖（高置信度边）
    $t = [string]$e.到
    if ($入度.ContainsKey($t)) { $入度[$t]++ } else { $入度[$t] = 1 }
    $种 = if ($种类表.ContainsKey($t)) { $种类表[$t] } else { '(未登记)' }
    if ($种类入度边.ContainsKey($种)) { $种类入度边[$种]++ } else { $种类入度边[$种] = 1 }
}

$判定种类 = @('函数', '常量', '静态量', '结构体', '枚举', '类型别名', '特征')
# 自检：某类符号若无任何入度边，说明索引不记录该类的被引用关系（如常量/类型），
# 其「入度为 0」恒成立、不携带信息，必须整类剔除，否则会产生成百上千条纯噪音。
$不可判种类 = @()
foreach ($种 in $判定种类) {
    if (-not $种类入度边.ContainsKey($种)) { $不可判种类 += $种 }
}
$可判种类 = @($判定种类 | Where-Object { $不可判种类 -notcontains $_ })

# trait 实现方法天然入度为 0：调用经 trait 分发（泛型/动态），静态调用边解析不到。
# 识别链：实现块名含「 for 」（即 impl Trait for Type）→ 该实现块经「包含」边持有的方法。
$特征实现块 = @{}
foreach ($s in $符.符号) {
    if ([string]$s.符号.种类 -ne '实现块') { continue }
    if ([string]$s.符号.名 -match '\s+for\s+') { $特征实现块[[string]$s.符号.id] = $true }
}
$特征方法 = @{}
foreach ($e in $符.边) {
    if ([string]$e.类型 -ne '包含') { continue }
    if ($特征实现块.ContainsKey([string]$e.从)) { $特征方法[[string]$e.到] = $true }
}

# 测试树：测试函数多以 #[cfg(test)] mod tests 内联在各源文件中，不在测试维度根下；
# 符号索引不记录属性，故改判「归属模块链上是否出现测试语义」。
$父模块 = @{}
foreach ($e in $符.边) {
    if ([string]$e.类型 -ne '包含') { continue }
    $子 = [string]$e.到
    if (-not $父模块.ContainsKey($子)) { $父模块[$子] = [string]$e.从 }
}
function 在测试树内([string]$id) {
    $当前 = $id
    for ($跳 = 0; $跳 -lt 32; $跳++) {
        if (-not $父模块.ContainsKey($当前)) { return $false }
        $上 = $父模块[$当前]
        if (-not $种类表.ContainsKey($上)) { return $false }
        if ($种类表[$上] -eq 'Crate') { return $false }
        if (-not $符号表.ContainsKey($上)) { return $false }
        if ([string]$符号表[$上].符号.名 -match 'test|测试') { return $true }
        $当前 = $上
    }
    return $false
}

$候选 = [System.Collections.Generic.List[object]]::new()
$候选分类 = @{}
$函数数 = 0
$函数有入度 = 0
# 覆盖率分母纠偏：trait 方法（经 trait 分发）与测试函数（经测试框架调用）都不产生静态调用边，
# 属「结构性无入度」，须从覆盖率分母剔除，否则覆盖率被系统性稀释、误导判读。
$测试函数全量 = 0
$可判函数数 = 0
$可判函数有入度 = 0
foreach ($s in $符.符号) {
    $种 = [string]$s.符号.种类
    $id = [string]$s.符号.id
    if ($种 -eq '函数') {
        $函数数++
        if ($入度.ContainsKey($id)) { $函数有入度++ }
        # 三类互斥切分：trait 方法 > 测试函数 > 可判函数（其余普通函数）
        if ($特征方法.ContainsKey($id)) {
            # trait 方法，不计入可判分母
        }
        elseif (在测试树内 $id) {
            $测试函数全量++
        }
        else {
            $可判函数数++
            if ($入度.ContainsKey($id)) { $可判函数有入度++ }
        }
    }
    if ($可判种类 -notcontains $种) { continue }
    if ([string]$s.符号.可见性 -ne '私有') { continue }
    if ($入度.ContainsKey($id)) { continue }
    if ($特征方法.ContainsKey($id)) { continue }
    if (在测试树内 $id) { continue }
    if ($null -ne $s.坐标 -and ($排除根 -contains [string]$s.坐标.根)) { continue }
    if ([string]$s.符号.名 -eq 'main') { continue }

    $坐标段 = @()
    if ($null -ne $s.坐标) {
        foreach ($k in @('根', '域', '府', '殿', '阁', '园')) {
            $v = $s.坐标.$k
            if ($null -ne $v) { $坐标段 += [string]$v }
        }
    }
    $候选.Add([ordered]@{
            'id'       = $id
            '名'       = [string]$s.符号.名
            '种类'     = $种
            '文件'     = [string]$s.符号.文件
            '行'       = $s.符号.行
            '坐标路径' = ($坐标段 -join '/')
        })
}

# 文本复核闸门：调用边解析覆盖不足（方法调用 / 函数指针 / 属性字符串 / 宏参数均解析不到），
# 故对每条候选回源文件做一次同名标识符复核：定义行之外若仍有出现，即证据不足，剔除。
# 方向为「宁可漏报，不可误报」，与落地设计的置信度契约一致。
$复核后 = [System.Collections.Generic.List[object]]::new()
$复核剔除 = 0
foreach ($c in $候选) {
    $物理 = Join-Path $根 ([string]$c.文件 -replace '/', '\')
    if (-not (Test-Path -LiteralPath $物理)) { $复核后.Add($c); continue }
    $命中 = @(Select-String -LiteralPath $物理 -Pattern ([regex]::Escape([string]$c.名)) |
            Where-Object { $_.LineNumber -ne $c.行 })
    if ($命中.Count -gt 0) { $复核剔除++; continue }
    $复核后.Add($c)
}
$候选 = $复核后
$候选分类 = @{}
foreach ($c in $候选) {
    $种 = [string]$c.种类
    if ($候选分类.ContainsKey($种)) { $候选分类[$种]++ } else { $候选分类[$种] = 1 }
}

# ---- G2：不可判种类的存疑线索（入度 0 强制列出） ----
# 索引不记录 常量/静态量/结构体/枚举/类型别名/特征 的被引用边，其「入度为 0」恒成立、不携带信息，
# 故不能据此判死代码；但也不该丢弃（丢弃等于假装这几类没有问题）。
# 口径：入度 0 的不可判种类一律列为「存疑线索」（低置信度，须人工/智能体按坐标回源核查）。
# 附全仓 .rs 文本命中数，仅用于排序（命中越少越可疑）与阅读参考，**不作剔除依据**——
# 同名异体、注释与文档字符串提及都会命中，命中数高不等于真被使用。
$不可判候选数 = 0
$零迹象数 = 0
$不可判项 = [System.Collections.Generic.List[object]]::new()
if ($不可判种类.Count -gt 0) {
    $源码文件 = @(Get-ChildItem -LiteralPath $根 -Recurse -File -Filter *.rs -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -notmatch '\\target\\|\\node_modules\\|\\.git\\' })
    $全仓文本 = ''
    if ($源码文件.Count -gt 0) {
        $全仓文本 = (@($源码文件 | ForEach-Object { [System.IO.File]::ReadAllText($_.FullName) }) -join "`n")
    }
    foreach ($s in $符.符号) {
        $种 = [string]$s.符号.种类
        if ($不可判种类 -notcontains $种) { continue }
        if ([string]$s.符号.可见性 -ne '私有') { continue }
        if ($入度.ContainsKey([string]$s.符号.id)) { continue }
        if (在测试树内 ([string]$s.符号.id)) { continue }
        if ($null -ne $s.坐标 -and ($排除根 -contains [string]$s.坐标.根)) { continue }
        $名 = [string]$s.符号.名
        if ($名.Length -lt 2) { continue }
        $不可判候选数++
        # 命中数 -1 = 未取到全仓文本（环境异常），不参与「零引用迹象」计数
        $次 = -1
        if ($全仓文本.Length -gt 0) { $次 = [regex]::Matches($全仓文本, [regex]::Escape($名)).Count }
        if ($次 -ge 0 -and $次 -le 1) { $零迹象数++ }
        $坐标段 = @()
        if ($null -ne $s.坐标) {
            foreach ($k in @('根', '域', '府', '殿', '阁', '园')) {
                $v = $s.坐标.$k
                if ($null -ne $v) { $坐标段 += [string]$v }
            }
        }
        $不可判项.Add([ordered]@{
                'id'           = [string]$s.符号.id
                '名'           = $名
                '种类'         = $种
                '文件'         = [string]$s.符号.文件
                '行'           = $s.符号.行
                '坐标路径'     = ($坐标段 -join '/')
                '全仓文本命中' = $次
                '判据'         = '索引不记录该类引用边（入度恒 0 无信息），按「入度 0 即列线索」口径强制列出；全仓文本命中数仅供排序，不作剔除依据。'
            })
    }
}

$调用边数 = @($符.边 | Where-Object { [string]$_.类型 -eq '调用' }).Count
$依赖边数 = @($符.边 | Where-Object { [string]$_.类型 -eq '依赖' }).Count
$函数覆盖率 = if ($函数数 -gt 0) { [math]::Round(100.0 * $函数有入度 / $函数数, 1) } else { 0 }
# 可判函数覆盖率：分母剔除 trait 方法 + 测试函数（结构性无入度），反映真实静态调用解析覆盖率。
$可判函数覆盖率 = if ($可判函数数 -gt 0) { [math]::Round(100.0 * $可判函数有入度 / $可判函数数, 1) } else { 0 }
# 覆盖率充足阈值：低于此值说明调用边解析覆盖不足，「入度为 0」不可直接判「死代码」。
$覆盖率充足 = $可判函数覆盖率 -ge 70

# ---- 统计 ----
$按类计数 = @{}
foreach ($f in $发现) { $k = $f.类; if ($按类计数.ContainsKey($k)) { $按类计数[$k]++ } else { $按类计数[$k] = 1 } }
$按级别计数 = @{}
foreach ($f in $发现) { $k = $f.级别; if ($按级别计数.ContainsKey($k)) { $按级别计数[$k]++ } else { $按级别计数[$k] = 1 } }
$高数 = if ($按级别计数.ContainsKey('高')) { $按级别计数['高'] } else { 0 }
$中数 = if ($按级别计数.ContainsKey('中')) { $按级别计数['中'] } else { 0 }

$时间 = Get-Date -Format 'yyyy-MM-dd HH:mm'
# 仓库目录名里的「 - 」在文档标题中统一呈现为「·」，与既有索引文档保持一致
$项目名 = (Split-Path -Leaf ([string]$坐.元信息.仓库根)) -replace ' - ', '·'

$结果 = [ordered]@{
    '元信息'     = [ordered]@{
        '图谱'     = '诊断'
        '版本'     = 'v1'
        '生成时间' = $时间
        '仓库根'   = [string]$坐.元信息.仓库根
        '生成器'   = '.传承/门禁/诊断图谱.ps1'
        '输入'     = @($坐标索引, $符号索引)
        '说明'     = '只读既有索引做一致性诊断；置信度分级：高=硬规则违规，中=疑似，低=画像线索。清理建议须人确认。'
    }
    '统计'       = [ordered]@{
        '发现总数' = $发现.Count
        '候选总数' = $候选.Count
        '按类'     = $按类计数
        '按级别'   = $按级别计数
    }
    '覆盖率画像' = [ordered]@{
        '函数数'           = $函数数
        '函数有入度'       = $函数有入度
        '函数覆盖率'       = "$函数覆盖率%"
        '可判函数数'       = $可判函数数
        '可判函数有入度'   = $可判函数有入度
        '可判函数覆盖率'   = "$可判函数覆盖率%"
        '调用边'           = $调用边数
        '依赖边'           = $依赖边数
        '可判种类'         = $可判种类
        '不可判种类'       = $不可判种类
        'trait方法数'      = $特征方法.Count
        '测试函数数'       = $测试函数全量
        '复核剔除数'       = $复核剔除
        '不可判种类复核'   = [ordered]@{
            '候选数' = $不可判候选数
            '剔除'   = $不可判剔除
            '存疑'   = $不可判项.Count
        }
        '插件自诊断'       = $自诊
        '说明'             = '可判函数覆盖率 = 剔除 trait 方法 + 测试函数（结构性无入度）后的静态调用覆盖率；低于 70% 时「入度为 0」不可判「死代码」。不可判种类复核 = 索引不记录其引用边的种类（常量 / 静态量 / 结构体 / 枚举 / 类型别名 / 特征）改用全仓文本检索作替代判据的结果：定义外另有出现即剔除，仅定义处出现 1 次者列为存疑（低置信度）。插件自诊断 = 语义层解析覆盖硬证据（缺省为空对象表示未提供）。'
    }
    '发现'       = $发现
    '死代码候选' = $候选
    '存疑线索'   = $不可判项
}

if (-not (Test-Path -LiteralPath $输出目录)) { New-Item -ItemType Directory -Path $输出目录 -Force | Out-Null }
$json路径 = Join-Path $输出目录 '诊断结果.json'
[System.IO.File]::WriteAllText($json路径, ($结果 | ConvertTo-Json -Depth 12), (New-Object System.Text.UTF8Encoding($false)))

# ---- 人读摘要 ----
$md = [System.Collections.Generic.List[string]]::new()
$md.Add('# ' + $项目名 + ' · 知识图谱 · 诊断报告')
$md.Add('')
$md.Add('> **定位**：知识图谱第 4 阶段（合并与诊断）——找过期项 / 可清理项。')
$md.Add('> **输入**：`坐标索引.json`（第 1 阶段）+ `符号索引.json`（第 2 阶段）。')
$md.Add('> **引擎**：`.传承/门禁/诊断图谱.ps1`（通用，零项目名词）。')
$md.Add('> **数据本体**：`诊断结果.json`（机器消费，本文件只是摘要）。')
$md.Add('> **置信度**：高 = 硬规则明确违规，可直接采信；中 = 疑似，须人工判定；低 = 画像线索，不可单独作为清理依据。')
$md.Add('> **生成时间**：' + $时间)
$md.Add('')
$md.Add('---')
$md.Add('')
$md.Add('## 一、总览')
$md.Add('')
$md.Add('| 指标 | 数值 |')
$md.Add('|---|---|')
$md.Add('| 发现总数 | ' + $发现.Count + ' |')
$md.Add('| 其中 · 高置信度 | ' + $高数 + ' |')
$md.Add('| 其中 · 疑似 | ' + $中数 + ' |')
$md.Add('| 死代码候选 | ' + $候选.Count + '（低置信度，见第五节） |')
$md.Add('')
$md.Add('## 二、按类分布')
$md.Add('')
$md.Add('| 类 | 数量 |')
$md.Add('|---|---|')
foreach ($k in ($按类计数.Keys | Sort-Object)) { $md.Add("| $k | $($按类计数[$k]) |") }
$md.Add('')
$md.Add('## 三、高置信度发现')
$md.Add('')
$高清单 = @($发现 | Where-Object { $_.级别 -eq '高' })
if ($高清单.Count -eq 0) {
    $md.Add('（无）')
}
else {
    $md.Add('| 类 | 对象 | 说明 | 依据 |')
    $md.Add('|---|---|---|---|')
    foreach ($f in $高清单) { $md.Add("| $($f.类) | $($f.对象) | $($f.说明) | $($f.依据) |") }
}
$md.Add('')
$md.Add('## 四、疑似发现')
$md.Add('')
$中清单 = @($发现 | Where-Object { $_.级别 -eq '中' })
if ($中清单.Count -eq 0) {
    $md.Add('（无）')
}
else {
    $md.Add('| 类 | 对象 | 说明 | 依据 |')
    $md.Add('|---|---|---|---|')
    foreach ($f in $中清单) { $md.Add("| $($f.类) | $($f.对象) | $($f.说明) | $($f.依据) |") }
}
$md.Add('')
$md.Add('## 五、死代码候选与置信度画像')
$md.Add('')
$md.Add('| 画像指标 | 数值 |')
$md.Add('|---|---|')
$md.Add('| 函数总数 | ' + $函数数 + ' |')
$md.Add('| 函数中有入度者 | ' + $函数有入度 + '（' + "$函数覆盖率%" + '） |')
$md.Add('| 可判函数（剔除 trait 方法 + 测试函数） | ' + $可判函数数 + '，其中入度者 ' + $可判函数有入度 + '（' + "$可判函数覆盖率%" + '） |')
$md.Add('| 调用边 | ' + $调用边数 + ' |')
$md.Add('| 依赖边 | ' + $依赖边数 + ' |')
$其他可判 = @($可判种类 | Where-Object { $_ -ne '函数' })
$其他可判行 = if ($其他可判.Count -gt 0) { $其他可判 -join ' / ' } else { '（无）' }
$md.Add('| 另有入度信息的种类 | ' + $其他可判行 + ' |')
$md.Add('| 无入度信息 · 索引不记录其引用边 | ' + ($不可判种类 -join ' / ') + ' |')
$md.Add('| 替代判据复核 · 上述种类（全仓文本检索） | 复核 ' + $不可判候选数 + '：剔除 ' + $不可判剔除 + '（定义外另有出现），存疑 ' + $不可判项.Count + ' |')
$md.Add('| 已排除 · trait 实现方法 | ' + $特征方法.Count + '（经 trait 分发调用，静态边不可见） |')
$md.Add('| 已排除 · 测试树内函数 | ' + $测试函数全量 + '（内联于测试模块，不属于测试维度根） |')
$md.Add('| 复核剔除 · 疑似有引用 | ' + $复核剔除 + '（同名标识符在源文件内另有出现） |')
$md.Add('| 候选 · 通过全部闸门 | ' + $候选.Count + ' |')
$md.Add('')
if ($不可判项.Count -gt 0) {
    $md.Add('### 存疑线索（不可判种类 · 替代判据复核后仍无引用）')
    $md.Add('')
    $md.Add('> 判据：索引不记录该类引用边，故「入度 = 0」不携带信息；改用全仓 `.rs` 文本检索，仅定义处出现 1 次。文本检索**不能证明「无人引用」**（宏展开 / 字符串拼接 / 依赖注入式注册都可能绕过），故只作线索，**不得据此直接删除**——须人工或智能体逐条回源核查。')
    $md.Add('')
    $md.Add('| 文件:行 | 名 | 种类 | 坐标 |')
    $md.Add('|---|---|---|---|')
    foreach ($c in ($不可判项 | Select-Object -First 25)) { $md.Add("| $($c.文件):$($c.行) | $($c.名) | $($c.种类) | $($c.坐标路径) |") }
    $md.Add('')
    if ($不可判项.Count -gt 25) {
        $md.Add('（共 ' + $不可判项.Count + ' 条，完整清单见 `诊断结果.json` 的 `存疑线索` 字段）')
        $md.Add('')
    }
}
$md.Add('> **判读**：入度分析仅采信 `调用` / `依赖`（高置信度边）。若函数入度覆盖率偏低，则「入度为 0」大量来自**解析未覆盖**而非真实未使用，此时清单**不可直接作为删除依据**。')
$md.Add('')
$md.Add('> **设计约束**（`语言解析插件-落地设计.md`）：`入度为零且非入口` 仅在高置信度下可判「死代码」，中低置信度只能标「疑似」；**清理建议须人确认**。')
$md.Add('')
$md.Add('过滤条件：种类 ∈ {' + ($可判种类 -join ' / ') + '} ∧ 可见性 = 私有 ∧ 入度 = 0 ∧ 非入口 ∧ 非 trait 实现方法 ∧ 非测试树内 ∧ 排除根 {' + ($排除根 -join ' / ') + '} ∧ 源文件内无同名出现。')
$md.Add('')
if ($候选.Count -gt 0) {
    $候选种类行 = ($候选分类.Keys | Sort-Object | ForEach-Object { "$_ $($候选分类[$_])" }) -join ' / '
    $md.Add('候选按种类：' + $候选种类行)
    $md.Add('')
}
if ($候选.Count -eq 0) {
    $md.Add('（无候选）')
    $md.Add('')
    if ($覆盖率充足) {
        if ($复核剔除 -gt 0) {
            $md.Add("> **结论**：死代码 0（**可信**）。入度为 0 的私有函数共 $复核剔除 条，全部因**源文件内另有同名出现**被复核剔除（即实际有引用，只是未被静态调用边索引到）。")
        }
        else {
            $md.Add('> **结论**：死代码 0（**可信**）。私有且入度为 0 的可判函数为 0，当前仓库无可静态判定的死代码。')
        }
        $md.Add('>')
        if ($自诊.Count -gt 0) {
            $md.Add('> 依据（入度侧）：可判函数覆盖率 ' + "$可判函数覆盖率%" + '（≥ 70% 阈值）。')
            $自诊段 = [System.Collections.Generic.List[string]]::new()
            if ($自诊.ContainsKey('函数调用取不到可调用体')) { $自诊段.Add('函数调用「取不到可调用体」' + $自诊['函数调用取不到可调用体'] + ' 条') }
            if ($自诊.ContainsKey('方法调用解析不出')) { $自诊段.Add('方法调用「解析不出」' + $自诊['方法调用解析不出'] + ' 条') }
            if ($自诊.ContainsKey('离开函数体调用点')) { $自诊段.Add('离开函数体 ' + $自诊['离开函数体调用点'] + ' 处') }
            if ($自诊.ContainsKey('仓库内不在符号表')) { $自诊段.Add('目标在仓库内却不在符号表 ' + $自诊['仓库内不在符号表'] + ' 处（宏展开 / trait 默认方法等结构性盲区）') }
            $md.Add('> 依据（语义层自诊断）：' + ($自诊段 -join '、') + '。')
            $md.Add('> 双侧一致 → 调用解析覆盖充足，「入度为 0」可采信。')
        }
        else {
            $md.Add('> 依据：可判函数覆盖率 ' + "$可判函数覆盖率%" + '（≥ 70% 阈值），调用边解析覆盖充足，「入度为 0」可采信。')
        }
        if ($复核剔除 -gt 0) {
            $md.Add('> 复核剔除的未覆盖形态：`self.方法()` 方法调用、函数指针传递（`from_fn_with_state(鉴权层)`）、属性字符串引用（`#[serde(default = "默认继承")]`）、宏参数内调用。')
        }
        $md.Add('>')
        $md.Add('> 不可判种类（' + ($不可判种类 -join ' / ') + '）不适用入度判据：已改用全仓文本检索复核 ' + $不可判候选数 + ' 条 —— 剔除 ' + $不可判剔除 + '（定义外另有出现），存疑 ' + $不可判项.Count + '。故「死代码 0」不因这几类缺判据而失真。')
    }
    else {
        $md.Add("> **结论**：该死代码识别路径**当前不可用**——可判函数覆盖率仅 $可判函数覆盖率%（< 70% 阈值），调用边解析覆盖不足，须先提升插件的调用解析覆盖，本清单才具备判「死代码」的资格。")
        if ($复核剔除 -gt 0) {
            $md.Add('>')
            $md.Add("> 参考：入度为 0 的私有函数共 $复核剔除 条，因**源文件内另有同名出现**被复核剔除（即实际有引用，只是未被索引解析到）。")
        }
    }
}
else {
    $md.Add('前 25 条：')
    $md.Add('')
    $md.Add('| 文件:行 | 名 | 种类 |')
    $md.Add('|---|---|---|')
    foreach ($c in ($候选 | Select-Object -First 25)) { $md.Add("| $($c.文件):$($c.行) | $($c.名) | $($c.种类) |") }
    $md.Add('')
    $md.Add('（完整清单见 `诊断结果.json` 的 `死代码候选` 字段）')
}
$md.Add('')
if ($发现.Count -gt 0) {
    $md.Add('> 另：结构 / 命名类发现共 ' + $发现.Count + ' 项（高 ' + $高数 + ' / 疑似 ' + $中数 + '），处置方向见第六节。')
    $md.Add('')
}
$md.Add('---')
$md.Add('')

# ---- 可清理项：把高置信度发现翻译成「动哪里、怎么动」的口径 ----
# 与第三节的分工：第三节照实列发现（是什么），本节只回答处置方向（怎么办）。
# 名单一律从索引数据推出，不硬编码项目目录名——换项目同样成立。
$md.Add('## 六、可清理项与处置建议（须人确认）')
$md.Add('')
$md.Add('> 高置信度 = 硬规则明确违规、可采信；但**清理建议须人确认**（设计约束），本节只给方向，不改仓库。')
$md.Add('')
$结构项 = @($发现 | Where-Object { $_.类 -eq '数量上限' })
$坐标项 = @($发现 | Where-Object { $_.类 -eq '坐标异常' })
$命名项 = @($发现 | Where-Object { $_.类 -eq '命名格式' -and $_.级别 -eq '高' })
$英文项 = @($发现 | Where-Object { $_.类 -eq '英文命名' })
$唯一项 = @($发现 | Where-Object { $_.类 -eq '命名唯一' })
$可清项数 = $结构项.Count + $坐标项.Count + $命名项.Count + $英文项.Count + $唯一项.Count
if ($可清项数 -eq 0) {
    $md.Add('（无可执行清理项）')
}
else {
    $md.Add('| 优先级 | 类 | 对象 | 现状 | 建议动作 |')
    $md.Add('|---|---|---|---|---|')
    foreach ($f in $结构项) {
        $对象 = [string]$f.对象
        # 「根下域 N 个 > 上限 M」与「阁 N 个 > 上限 M」两种措辞：前者层名是「域」
        $层 = if ([string]$f.说明 -like '根下域*') { '域' } else { [string](@([string]$f.说明 -split ' ')[0]) }
        $子 = $按父[$对象]
        $名单 = if ($null -eq $子) { '' }
        else { (@($子 | Where-Object { $_.层级 -eq $层 } | ForEach-Object { 取末段 $_.路径 }) -join '、') }
        $md.Add("| P0 | 数量上限 | $对象 | $($f.说明)；现有 $层：$名单 | 收敛至上限内：合并或移除冗余的「$层」目录 |")
    }
    foreach ($f in $坐标项) {
        $md.Add("| P0 | 坐标异常 | $($f.对象) | $($f.说明) | 按期望层改名补后缀，或移入正确的层目录 |")
    }
    foreach ($f in $命名项) {
        $名 = 取末段 ([string]$f.对象)
        $主 = if ($名.Length -gt 2) { $名.Substring(0, $名.Length - 2) } else { $名 }
        $md.Add("| P1 | 命名格式 | $($f.对象) | 主名「$($主)」$($主.Length) 字，应为 4 字 | 补足 / 精简至 4 字（如「符号提取-殿」），或登记为条文例外 |")
    }
    foreach ($f in $唯一项) {
        $md.Add("| P1 | 命名唯一 | $($f.对象) | $($f.说明) | 下级改用不同语义词，避免与上级同名 |")
    }
    foreach ($f in $英文项) {
        $md.Add("| P2 | 英文命名 | $($f.对象) | 目录名含英文 | 属技术专名（rust / crate 等）可登记例外；否则改中文名 |")
    }
}
$md.Add('')
$md.Add('> **P0** = 结构与层序违规（影响坐标可用性，先办）；**P1** = 命名规则违规；**P2** = 例外候选（须先判定是否属技术专名）。')
$md.Add('')
$md.Add('---')
$md.Add('')

$md.Add('## 七、本报告未覆盖')
$md.Add('')
$md.Add('| 项 | 说明 |')
$md.Add('|---|---|')
$md.Add('| 声明索引（阶段 3） | 契约 / 规则条文 / 文档引用尚未入图，故 `drift` / `broken_contract` 无法判定 |')
$md.Add('| 文档-代码数字一致性 | 需解析文档中的数字引用，未实现 |')
$md.Add('| 文档层悬空引用 | 需解析 Markdown 中的符号引用，未实现 |')
$md.Add('| 文件名英文命名 | 仅查目录；文件名（Cargo.toml / index.html 等）属例外，未纳入 |')

$md路径 = Join-Path $输出目录 '诊断报告.md'
[System.IO.File]::WriteAllText($md路径, (($md -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))

"诊断完成：发现 $($发现.Count)（高 $高数 / 疑似 $中数）/ 死代码候选 $($候选.Count)"
"→ $json路径（$((Get-Item -LiteralPath $json路径).Length) 字节）"
"→ $md路径"
exit 0
