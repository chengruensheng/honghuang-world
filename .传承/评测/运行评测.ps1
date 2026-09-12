# 运行评测.ps1 —— 外部标尺（对外公开可复现的任务库 + 标准化评测入口）
#
# 存在意义：破解「自证闭环」——系统的验收结论由被验收方（LLM / 五层驱动流程）自己生产，
# 用户拿到的「通过」可能只是模型的自我声明，缺乏独立、机器可判真伪的真相来源。
# 本入口对 任务库.json 中的每个标准任务：
#   1) （真实模式）切到干净评测工作区 → 发布任务 → 驱动五层至空闲；
#   2) **独立**在工作区上执行外部判据（文件内容正则 + 独立 cargo test），完全不读系统的验收结论；
#   3) 把「系统自述终态」与「外部判据结论」并列落盘——二者不一致即「自证失效」的实证。
# 所有原始输出（控制台全程 / 会话原文 / 判据命令 stdout）落盘，禁止文档转述数字。
#
# 用法：
#   pwsh -File .\.传承\评测\运行评测.ps1 -工作区 "D:\产物目录"                  # 离线判据（不调后端）
#   pwsh -File .\.传承\评测\运行评测.ps1 -真实                                 # 真实全链（缺省用项目内沙箱 ./评测沙箱）
#   pwsh -File .\.传承\评测\运行评测.ps1 -真实 -任务id fib,sort                # 只跑指定任务
# 注意：-工作区 必须与智能体进程**共享可见**。实测：项目外路径可能被运行环境沙箱拒绝写，
#       用户 TEMP 虽可写但对评测进程不可见（进程级文件隔离）——两者都会让判据落空而误判系统。
param(
    [string]$工作区 = "",
    [string]$后端 = "http://localhost:8321",
    [string[]]$任务id = @(),
    [switch]$真实,
    [switch]$允许脏看板,
    [int]$轮询超时秒 = 2400,
    [int]$总超时秒 = 7200,
    [int]$轮询间隔秒 = 2
)

$ErrorActionPreference = "Stop"

# 前置硬校验：本脚本依赖 PS7 的字节级 UTF-8 解码（Invoke-WebRequest 响应 Content 为 byte[]）。
# PS5.1 会按本地代码页误解码中文 JSON，导致「就绪」判断失真，故直接拒绝运行。
if ($PSVersionTable.PSVersion.Major -lt 7) {
    Write-Host "✗ 检测到 PowerShell $($PSVersionTable.PSVersion.ToString())（低于 7），本脚本需 pwsh 7+。" -ForegroundColor Red
    Write-Host "  请改用：pwsh -ExecutionPolicy Bypass -File .\.传承\评测\运行评测.ps1" -ForegroundColor Yellow
    exit 1
}

# 项目根：本脚本位于 .传承/评测/，向上回溯 2 层即项目根
$项目根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$任务库路径 = Join-Path $PSScriptRoot '任务库.json'

# ============ 原始产出目录（结论落盘原始 stdout，禁止文档转述数字） ============
$评测时间戳 = (Get-Date).ToString('yyyyMMdd-HHmmss')
$输出根 = Join-Path $PSScriptRoot '原始输出'
$评测目录 = Join-Path $输出根 "评测-$评测时间戳"
if (-not (Test-Path $评测目录)) { New-Item -ItemType Directory -Path $评测目录 -Force | Out-Null }
try { Start-Transcript -Path (Join-Path $评测目录 '控制台.txt') | Out-Null } catch { }

# 最近一次 HTTP 响应原文（由 调用Json 填充）：供原样落盘，杜绝二次转述
$script:最近原文 = $null

function 存原文($名, $文本) {
    if ($null -eq $文本) { $文本 = '' }
    Set-Content -Path (Join-Path $评测目录 $名) -Value $文本 -Encoding UTF8
}

function 写结果($结果对象) {
    # 结构化结果同时写本次目录与「最新」指针，供门禁/文档引用，避免文档手写指标
    $文本 = $结果对象 | ConvertTo-Json -Depth 8
    Set-Content -Path (Join-Path $评测目录 '评测结果.json') -Value $文本 -Encoding UTF8
    Set-Content -Path (Join-Path $PSScriptRoot '评测结果.json') -Value $文本 -Encoding UTF8
}

function 写失败结果($原因) {
    Write-Host "✗ $原因" -ForegroundColor Red
    写结果 ([ordered]@{
        生成时间 = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK')
        生成脚本 = '.传承/评测/运行评测.ps1'
        结论 = '失败'
        原因 = $原因
        后端 = $后端
        工作区 = $工作区
        原始数据目录 = "原始输出/评测-$评测时间戳"
        原始数据 = [ordered]@{ 控制台 = '控制台.txt' }
    })
    try { Stop-Transcript } catch { }
    exit 1
}

function 调用Json($方法, $路径, $体 = $null) {
    # 统一按 UTF-8 解码（PS7 下 Content 即 byte[]），规避中文键/值误解码
    if ($null -eq $体) {
        $响应 = Invoke-WebRequest -Uri "$后端$路径" -Method $方法 -UseBasicParsing
    } else {
        $响应 = Invoke-WebRequest -Uri "$后端$路径" -Method $方法 -Body ($体 | ConvertTo-Json -Compress) -ContentType "application/json; charset=utf-8" -UseBasicParsing
    }
    $原始字节 = $null
    if ($响应.Content -is [byte[]]) {
        $原始字节 = $响应.Content
    } elseif ($null -ne $响应.PSObject.Properties['RawContentStream'] -and $null -ne $响应.RawContentStream) {
        $原始字节 = $响应.RawContentStream.ToArray()
    }
    $文本 = if ($null -ne $原始字节) { [System.Text.Encoding]::UTF8.GetString($原始字节) } else { [string]$响应.Content }
    $script:最近原文 = $文本
    if ([string]::IsNullOrWhiteSpace($文本)) { return $null }
    $文本 | ConvertFrom-Json
}

# ============ 读任务库 ============
Write-Host "========== 外部标尺 · 标准任务库评测 ==========" -ForegroundColor Cyan
if (-not (Test-Path $任务库路径)) { 写失败结果 "任务库不存在：$任务库路径" }
try { $库 = Get-Content $任务库路径 -Raw | ConvertFrom-Json } catch { 写失败结果 "任务库不可解析：$($_.Exception.Message)" }
$任务清单 = @($库.任务)
if ($任务id.Count -gt 0) { $任务清单 = @($任务清单 | Where-Object { $任务id -contains $_.id }) }
if ($任务清单.Count -eq 0) { 写失败结果 "无待评测任务（任务库为空或 -任务id 无匹配）" }
Write-Host "任务库：$($任务清单.Count) 个任务（$(($任务清单 | ForEach-Object { $_.id }) -join '、')）"

# ============ 工作区解析与安全校验 ============
function 校验工作区安全($路径) {
    # 评测要清空工作区，必须拒绝任何「疑似项目根」的目标，避免误删源码
    $规范 = [System.IO.Path]::GetFullPath($路径)
    if ($规范.TrimEnd('\') -eq $项目根.TrimEnd('\')) { throw "评测工作区不得为项目根：$规范" }
    if (Test-Path (Join-Path $规范 '.git')) { throw "评测工作区不得是 git 仓库根（含 .git）：$规范" }
    if (Test-Path (Join-Path $规范 'Cargo.toml')) { throw "评测工作区不得含顶层 Cargo.toml（疑似项目根）：$规范" }
    return $规范
}

$模式 = if ($真实) { '真实全链' } else { '离线判据' }
if ([string]::IsNullOrWhiteSpace($工作区)) {
    if (-not $真实) { 写失败结果 "离线判据须用 -工作区 指定产物目录（真实模式可缺省，将自动选用项目内沙箱）" }
    # 真实模式缺省沙箱必须落在**项目目录内**，原因（第四轮实测教训，代价是被测对象两次无效重跑）：
    #   1) 项目外路径（F:\临时工作区）被运行环境沙箱直接拒绝写；
    #   2) 用户 TEMP 虽可写，但智能体进程的写入对评测进程**不可见**（进程级文件隔离），
    #      判据会读到空目录、把「基座不可见」误判成「系统没做完」；
    #   3) 项目目录内的写入是跨进程共享可见的（第二轮产物落 ./工作区，评测进程可正常读到）。
    # 项目内沙箱已在根 Cargo.toml 的 workspace.exclude 中登记，不会被纳入本工作区。
    $工作区 = Join-Path $项目根 '评测沙箱'
    Write-Host "未指定工作区，自动选用项目内沙箱：$工作区"
}
if (-not (Test-Path $工作区)) {
    if ($真实) { New-Item -ItemType Directory -Path $工作区 -Force | Out-Null }
    else { 写失败结果 "工作区不存在：$工作区" }
}
try { $工作区绝对 = 校验工作区安全 $工作区 } catch { 写失败结果 $_.Exception.Message }
Write-Host "模式：$模式 · 后端：$后端"
Write-Host "工作区：$工作区绝对"
Write-Host ""

# ============ 前置自检（真实模式） ============
if ($真实) {
    # 工作区可写自检（fail-fast）：智能体产物必须落在此目录；若不可写则产物为空，
    # 外部判据必然落空并把「基座故障」误判成「系统没做完」（第三轮实测教训）。
    $探测文件 = Join-Path $工作区绝对 '.评测写权限探测'
    try {
        Set-Content -Path $探测文件 -Value 'ok' -Encoding UTF8 -ErrorAction Stop
        Remove-Item -Path $探测文件 -Force -ErrorAction SilentlyContinue
    } catch {
        写失败结果 "评测工作区不可写（$工作区绝对）：$($_.Exception.Message)。请改用可写且与智能体进程共享可见的目录（缺省即项目内 ./评测沙箱），否则智能体产物无法落盘、外部判据必然落空"
    }
    Write-Host "工作区可写：是" -ForegroundColor Green

    try {
        $状态 = 调用Json GET "/api/dev/pilot/status"
        存原文 '驱动台状态.json' $script:最近原文
        if (-not $状态.就绪) { 写失败结果 '看板驱动台未装配（就绪=false），无法真实驱动' }
    } catch { 写失败结果 "无法连接后端 $后端：请先运行 .\启动.ps1 并接入 LLM" }
    Write-Host "驱动台就绪：是" -ForegroundColor Green

    # 看板洁净前置检查（fail-loud）：发布任务会触发「看板驱动台自动驱动一轮」，
    # 而驱动台按看板顺序挑**最早的可推进任务**——若看板上有遗留的未终态任务，
    # 被驱动的将是遗留任务而非本次发布的任务，评测结论（尤其「系统自述终态」）
    # 会张冠李戴、彻底失真。故默认拒绝在脏看板上跑，除非显式 -允许脏看板。
    if (-not $允许脏看板) {
        try {
            $前置看板 = @(调用Json GET "/api/board")
            存原文 '前置看板.json' $script:最近原文
            $终态集合 = @('清理完成', '已完成', '已取消')
            $未终态 = @($前置看板 | Where-Object { $终态集合 -notcontains [string]$_.status })
            if ($未终态.Count -gt 0) {
                $样例 = ($未终态 | Select-Object -First 3 | ForEach-Object { "#$($_.id)=$($_.status)" }) -join '、'
                写失败结果 "看板存在 $($未终态.Count) 个未终态任务（$样例）：自动驱动会插队驱动遗留任务，污染评测结论。请先清理看板至全部终态，或用干净持久化目录另起实例；确要冒险可用 -允许脏看板"
            }
        } catch { 写失败结果 "读取看板失败（$后端）：$($_.Exception.Message)" }
        Write-Host "看板洁净：是（无未终态遗留任务）" -ForegroundColor Green
    } else {
        Write-Host "看板洁净：跳过（-允许脏看板，结论可能被遗留任务污染）" -ForegroundColor Yellow
    }
}

# ============ 外部判据执行（独立于系统自述） ============
function 执行判据($任务, $根, $任务目录) {
    $明细 = @()
    $全过 = $true
    $序号 = 0
    foreach ($判据 in @($任务.判据)) {
        $序号++
        $项 = [ordered]@{ 序号 = $序号; 类型 = [string]$判据.类型; 通过 = $false }
        if ($判据.类型 -eq '文件内容正则') {
            $正则 = [string]$判据.正则
            $后缀 = '.rs'
            if ($null -ne $判据.glob -and ([string]$判据.glob) -match '\.(?<ext>\w+)$') { $后缀 = '.' + $Matches['ext'] }
            $命中文件 = @()
            foreach ($文件 in (Get-ChildItem -Path $根 -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -eq $后缀 })) {
                $内容 = Get-Content -Path $文件.FullName -Raw -ErrorAction SilentlyContinue
                if ($null -ne $内容 -and $内容 -match $正则) {
                    $命中文件 += $文件.FullName.Substring($根.Length).TrimStart('\')
                }
            }
            $项.正则 = $正则
            $项.命中文件 = @($命中文件)
            $项.通过 = ($命中文件.Count -gt 0)
        }
        elseif ($判据.类型 -eq '构建测试') {
            $清单 = Get-ChildItem -Path $根 -Recurse -File -Filter 'Cargo.toml' -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($null -eq $清单) {
                $项.说明 = '工作区内未找到 Cargo.toml，产物不可独立构建'
            } else {
                $crate目录 = $清单.DirectoryName
                $输出名 = "判据-$序号-cargo-test.txt"
                $输出文件 = Join-Path $任务目录 $输出名
                Push-Location $crate目录
                try {
                    # 用 PowerShell 原生重定向捕获 stdout+stderr，不依赖 cmd /c（受限环境可能拦截）
                    $判据输出 = & cargo test 2>&1
                    $码 = $LASTEXITCODE
                    $判据输出 | Out-File -FilePath $输出文件 -Encoding utf8
                } finally { Pop-Location }
                $行们 = Get-Content $输出文件 -ErrorAction SilentlyContinue
                $有通过 = [bool]($行们 | Select-String -Pattern 'test result: ok\.' -Quiet)
                $有失败 = [bool]($行们 | Select-String -Pattern 'test result: FAILED\.|error\[E0' -Quiet)
                $项.命令 = [string]$判据.命令
                $项.crate目录 = $crate目录.Substring($根.Length).TrimStart('\')
                $项.退出码 = $码
                $项.原始输出 = $输出名
                $项.通过 = ($码 -eq 0 -and $有通过 -and -not $有失败)
            }
        }
        else {
            $项.说明 = "未知判据类型：$($判据.类型)"
        }
        if (-not $项.通过) { $全过 = $false }
        $明细 += $项
    }
    return [ordered]@{ 通过 = $全过; 明细 = $明细 }
}

# ============ 逐任务评测 ============
$终态集合 = @('清理完成', '已完成', '已取消')   # 终态：走完五层或显式取消
$任务记录 = @()
$外部通过数 = 0
$自述通过数 = 0
$自证失效数 = 0

foreach ($任务 in $任务清单) {
    $任务目录 = Join-Path $评测目录 ("任务-{0}" -f $任务.id)
    if (-not (Test-Path $任务目录)) { New-Item -ItemType Directory -Path $任务目录 -Force | Out-Null }
    Write-Host "[$($任务.id)] $($任务.标题)（难度 $($任务.难度)）" -ForegroundColor Yellow

    $系统自述 = $null
    if ($真实) {
        # 每个任务从干净工作区起步：产物必须自证完备，不得借前次残留
        Get-ChildItem -Path $工作区绝对 -Force -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
        # 必须用 POST /api/workspace（项目工作区）：它同时重装配「智能体沙箱」与「看板驱动台执行器」；
        # POST /api/dev/workspace 只重装配智能体沙箱——实测五层驱动产物仍落 default.toml 的 dev_workspace，
        # 判据会查错目录（首轮曾由此得出失真的 0/3）。切换响应含规范化绝对路径，据此校准判据目录。
        try {
            $切换 = 调用Json POST "/api/workspace" @{ 工作区 = $工作区绝对 }
            存原文 '工作区切换.json' $script:最近原文
            if ($null -ne $切换 -and -not [string]::IsNullOrWhiteSpace([string]$切换.工作区)) {
                $工作区绝对 = [System.IO.Path]::GetFullPath([string]$切换.工作区)
            }
        } catch { Write-Host "  工作区切换失败：$($_.Exception.Message)" -ForegroundColor Red }

        $发布 = 调用Json POST "/api/board" @{ title = $任务.标题; description = $任务.描述 }
        # POST /api/board 返回裸数字任务 id（非对象），勿写 $发布.id（会得空值）
        $发布任务号 = $发布
        Write-Host "  任务 #$发布任务号 已发布，自主驱动中…"

        # 驱动语义：一次 drain 只推进**一个阶段**（五层 = 木→火→土→金→水）。
        # 只 drain 一次会停在第一层之后（实测停在「待大罗金仙实现」），
        # 故必须循环 drain 直到任务达到终态；连续 3 轮无推进则判定卡住、停止空转。
        $驱动起始 = Get-Date
        $上轮终态 = ''
        $停滞次数 = 0
        $轮次 = 0
        $当前终态 = $null
        while ($true) {
            $轮次++
            Start-Sleep -Seconds 3
            $受理成功 = $false
            for ($重试 = 1; $重试 -le 3; $重试++) {
                try { 调用Json POST "/api/dev/pilot/drain" @{ 上限 = 60 } | Out-Null; $受理成功 = $true; break }
                catch { Start-Sleep -Seconds ($轮询间隔秒 * 2) }
            }
            # 等本轮驱动落定
            $轮询起始 = Get-Date
            while ($true) {
                Start-Sleep -Seconds $轮询间隔秒
                $状态 = 调用Json GET "/api/dev/pilot/status"
                if (-not $状态.运行中) { break }
                if (((Get-Date) - $轮询起始).TotalSeconds -gt $轮询超时秒) { Write-Host "  单轮驱动轮询超时（$轮询超时秒 秒）" -ForegroundColor Red; break }
            }
            # 查当前任务终态（系统自述）
            try {
                $看板 = @(调用Json GET "/api/board")
                存原文 ("任务-{0}-看板.json" -f $任务.id) $script:最近原文
                $卡 = $看板 | Where-Object { $_.id -eq $发布任务号 } | Select-Object -First 1
                if ($null -ne $卡) { $当前终态 = [string]$卡.status }
            } catch { }
            Write-Host "    [轮 $轮次] 任务状态：$当前终态"
            if ($终态集合 -contains $当前终态) { break }
            if (-not $受理成功) { $停滞次数++ } elseif ($当前终态 -eq $上轮终态) { $停滞次数++ } else { $停滞次数 = 0 }
            $上轮终态 = $当前终态
            if ($停滞次数 -ge 3) { Write-Host "  连续 3 轮状态无推进（$当前终态），停止驱动以免空转" -ForegroundColor Yellow; break }
            if (((Get-Date) - $驱动起始).TotalSeconds -gt $总超时秒) { Write-Host "  任务总驱动超时（$总超时秒 秒），停止" -ForegroundColor Red; break }
        }
        $终态 = $当前终态
        $自述通过 = ($终态 -eq '已完成' -or $终态 -eq '清理完成')
        $系统自述 = [ordered]@{ 任务id = $发布任务号; 任务终态 = $终态; 自述通过 = $自述通过 }
        if ($自述通过) { $自述通过数++ }
        Write-Host "  系统自述终态：$终态（自述通过=$自述通过）"
    }

    # 外部判据：独立执行，不看系统结论
    $判据结果 = 执行判据 $任务 $工作区绝对 $任务目录
    if ($判据结果.通过) { $外部通过数++ }
    Write-Host "  外部判据：$(if ($判据结果.通过) { '通过' } else { '不通过' })" -ForegroundColor $(if ($判据结果.通过) { 'Green' } else { 'Red' })

    $不一致 = $false
    if ($真实 -and $null -ne $系统自述) {
        $不一致 = ($系统自述.自述通过 -and -not $判据结果.通过)
        if ($不一致) { $自证失效数++ }
    }

    $任务记录 += [ordered]@{
        id = $任务.id
        标题 = $任务.标题
        难度 = $任务.难度
        系统自述 = $系统自述
        外部判据 = $判据结果
        自述通过而外部不通过 = $不一致
    }
    Write-Host ""
}

# ============ 汇总与落盘 ============
$外部通过率 = if ($任务清单.Count -gt 0) { [math]::Round(100 * $外部通过数 / $任务清单.Count, 1) } else { 0 }
$自述通过率 = $null
if ($真实) { $自述通过率 = if ($任务清单.Count -gt 0) { [math]::Round(100 * $自述通过数 / $任务清单.Count, 1) } else { 0 } }

Write-Host "========== 评测汇总 ==========" -ForegroundColor Cyan
Write-Host ("外部判据通过率 = {0}%（{1}/{2}）" -f $外部通过率, $外部通过数, $任务清单.Count)
if ($真实) {
    Write-Host ("系统自述通过率 = {0}%（{1}/{2}）" -f $自述通过率, $自述通过数, $任务清单.Count)
    Write-Host ("自证失效数 = {0}（系统自称通过、外部判据却不通过——自证闭环的实证）" -f $自证失效数) -ForegroundColor $(if ($自证失效数 -gt 0) { 'Red' } else { 'Green' })
}
Write-Host ""

写结果 ([ordered]@{
    生成时间 = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK')
    生成脚本 = '.传承/评测/运行评测.ps1'
    结论 = '完成'
    模式 = $模式
    后端 = $后端
    工作区 = $工作区绝对
    任务总数 = $任务清单.Count
    外部判据通过数 = $外部通过数
    外部判据通过率 = $外部通过率
    系统自述通过数 = $(if ($真实) { $自述通过数 } else { $null })
    系统自述通过率 = $自述通过率
    自证失效数 = $(if ($真实) { $自证失效数 } else { $null })
    任务 = @($任务记录)
    原始数据目录 = "原始输出/评测-$评测时间戳"
    原始数据 = [ordered]@{ 控制台 = '控制台.txt' }
})
Write-Host "结构化结果已落盘：$($评测目录 | Split-Path -Leaf)/评测结果.json（最新指针：.传承/评测/评测结果.json）" -ForegroundColor Green
try { Stop-Transcript } catch { }
exit 0
