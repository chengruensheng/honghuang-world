# 运行评测.ps1 —— 外部标尺（对外公开可复现的任务库 + 标准化评测入口）
#
# 存在意义：破解「自证闭环」——系统的验收结论由被验收方（LLM / 五层驱动流程）自己生产，
# 用户拿到的「通过」可能只是模型的自我声明，缺乏独立、机器可判真伪的真相来源。
# 本入口对 任务库.json 中的每个标准任务：
#   1) （真实模式）切到干净评测工作区 → 发布任务 → 驱动五层至空闲；
#   2) **独立**在工作区上执行外部判据（文件内容正则 + 独立 cargo test + 注入评测方夹具），完全不读系统的验收结论；
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
    [bool]$自动终止卡住任务 = $true,
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
function 校验工作区安全($路径, $项目内沙箱) {
    # 评测（真实模式）要清空工作区，必须拒绝任何「疑似项目根」的目标，避免误删源码。
    # 注意：「顶层 Cargo.toml」只是**弱**启发式——产物自身就可能在根建 workspace 清单，
    # 上一轮残留的产物根 Cargo.toml 属正常。故它只在「真实模式 + 用户自选目录」这一真正
    # 有误删风险的情形下生效；离线判据（不清空）与项目内缺省沙箱一律放行（实测教训）。
    $规范 = [System.IO.Path]::GetFullPath($路径)
    if ($规范.TrimEnd('\') -eq $项目根.TrimEnd('\')) { throw "评测工作区不得为项目根：$规范" }
    if (Test-Path (Join-Path $规范 '.git')) { throw "评测工作区不得是 git 仓库根（含 .git）：$规范" }
    if ($真实 -and -not $项目内沙箱 -and (Test-Path (Join-Path $规范 'Cargo.toml'))) { throw "评测工作区不得含顶层 Cargo.toml（疑似项目根）：$规范。自选目录若确为可弃产物目录，请显式清理或改用缺省沙箱" }
    return $规范
}

$模式 = if ($真实) { '真实全链' } else { '离线判据' }
$缺省沙箱 = Join-Path $项目根 '评测沙箱'
if ([string]::IsNullOrWhiteSpace($工作区)) {
    if (-not $真实) { 写失败结果 "离线判据须用 -工作区 指定产物目录（真实模式可缺省，将自动选用项目内沙箱）" }
    # 真实模式缺省沙箱必须落在**项目目录内**，原因（第四轮实测教训，代价是被测对象两次无效重跑）：
    #   1) 项目外路径（F:\临时工作区）被运行环境沙箱直接拒绝写；
    #   2) 用户 TEMP 虽可写，但智能体进程的写入对评测进程**不可见**（进程级文件隔离），
    #      判据会读到空目录、把「基座不可见」误判成「系统没做完」；
    #   3) 项目目录内的写入是跨进程共享可见的（第二轮产物落 ./工作区，评测进程可正常读到）。
    # 项目内沙箱已在根 Cargo.toml 的 workspace.exclude 中登记，不会被纳入本工作区。
    $工作区 = $缺省沙箱
    Write-Host "未指定工作区，自动选用项目内沙箱：$工作区"
}
if (-not (Test-Path $工作区)) {
    if ($真实) { New-Item -ItemType Directory -Path $工作区 -Force | Out-Null }
    else { 写失败结果 "工作区不存在：$工作区" }
}
$是否项目内沙箱 = ([System.IO.Path]::GetFullPath($工作区).TrimEnd('\') -eq [System.IO.Path]::GetFullPath($缺省沙箱).TrimEnd('\'))
try { $工作区绝对 = 校验工作区安全 $工作区 $是否项目内沙箱 } catch { 写失败结果 $_.Exception.Message }
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
            $终态集合 = @('清理完成', '已完成', '已取消', '已确认无解')
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
function 取Crate名($文本) {
    # 从 Cargo.toml 取 crate 名：优先 [lib].name（集成测试的 use 路径名），否则 [package].name
    $段 = ''
    $包名 = $null
    $库名 = $null
    foreach ($行 in ($文本 -split "`r?`n")) {
        $t = $行.Trim()
        if ($t -match '^\[(?<s>[^\]]+)\]') { $段 = $Matches['s'].Trim(); continue }
        if ($t -match '^name\s*=\s*"([^"]+)"') {
            if ($段 -eq 'package' -and $null -eq $包名) { $包名 = $Matches[1] }
            elseif ($段 -eq 'lib' -and $null -eq $库名) { $库名 = $Matches[1] }
        }
    }
    if ($null -ne $库名) { return $库名 }
    return $包名
}

function 执行判据($任务, $根, $任务目录) {
    $明细 = @()
    $全过 = $true
    $标尺故障 = $false   # 判据根本没跑起来（评测基座问题）——与「判据不通过」严格区分
    $有产物缺失 = $false # 系统未交付可对照的产物——属对产物的判定，非基座故障
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
        elseif ($判据.类型 -eq '外部测试') {
            # 外部夹具（评测方提供，不属被测产物）：注入产物 crate 的 tests/ 后单独运行。
            # 与「构建测试」的关键差别：断言由**评测方**写死，被测系统无法用弱自测蒙混。
            $夹具名 = [string]$判据.夹具
            $夹具路径 = Join-Path (Join-Path $PSScriptRoot '夹具') $夹具名
            # 产物可能是「根 workspace 清单 + 成员包清单」两层（实测大罗金仙会在产物根新建
            # [workspace] Cargo.toml）。必须跳过纯 workspace 清单、选出真正的**包**清单，
            # 并优先带 [lib] 的包（夹具以 use 路径引用其导出项）。若误取 workspace 清单会解析
            # 不出 crate 名、产生空判据——在加压任务上还会伪装成「符合期望」，故显式挑选。
            $清单 = $null
            $crate名 = $null
            foreach ($候选 in @(Get-ChildItem -Path $根 -Recurse -File -Filter 'Cargo.toml' -ErrorAction SilentlyContinue)) {
                $清单文本 = Get-Content -Path $候选.FullName -Raw
                if ($清单文本 -notmatch '(?m)^\s*\[package\]') { continue }   # 跳过纯 workspace 清单
                $名 = 取Crate名 $清单文本
                if ([string]::IsNullOrWhiteSpace($名)) { continue }
                if ($清单文本 -match '(?m)^\s*\[lib\]') { $清单 = $候选; $crate名 = $名; break }
                if ($null -eq $清单) { $清单 = $候选; $crate名 = $名 }
            }
            $项故障 = $false          # 标尺故障：评测基座自己没跑起来（夹具缺失等），外部结论**无效**
            $本项产物缺失 = $false    # 产物缺失：系统未交付可注入的包——这是**对产物的判定**，不是基座故障
            if (-not (Test-Path $夹具路径)) {
                # 夹具缺失属**标尺故障**（评测方资产问题），不是对产物的判定——必须与
                # 「判据不通过」严格区分，否则加压任务上会被误读成「符合期望」（假达标）。
                $项.说明 = "夹具不存在：$夹具路径"
                $项故障 = $true
            } elseif ($null -eq $清单) {
                # 工作区无含 [package] 的清单 = 系统没有交付可对照的产物。这也是**对产物的判定**
                # （不通过），而非基座故障——实证：加压任务「整数精确一半」上系统正确识别需求
                # 数学无解、拒绝产出伪实现、工作区保持为空；若记成标尺故障，会把系统的正确行为
                # 误判成评测方问题并从分母剔除（本轮实测教训）。
                $项.说明 = '产物缺可注入的包（工作区内未找到含 [package] 的 Cargo.toml）——无产物可对照，判据不通过'
                $本项产物缺失 = $true
            } else {
                $crate目录 = $清单.DirectoryName
                # Rust 标识符不能含连字符：package 名 my-crate → use 路径 my_crate
                $标识符 = $crate名 -replace '-', '_'
                $注入正文 = (Get-Content -Path $夹具路径 -Raw).Replace('{{crate}}', $标识符)
                $测试目录 = Join-Path $crate目录 'tests'
                if (-not (Test-Path $测试目录)) { New-Item -ItemType Directory -Path $测试目录 -Force | Out-Null }
                Set-Content -Path (Join-Path $测试目录 '外部判据.rs') -Value $注入正文 -Encoding UTF8
                $输出名 = "判据-$序号-外部测试.txt"
                $输出文件 = Join-Path $任务目录 $输出名
                Push-Location $crate目录
                try {
                    $判据输出 = & cargo test --test 外部判据 2>&1
                    $码 = $LASTEXITCODE
                    $判据输出 | Out-File -FilePath $输出文件 -Encoding utf8
                } finally { Pop-Location }
                $行们 = Get-Content $输出文件 -ErrorAction SilentlyContinue
                $有通过 = [bool]($行们 | Select-String -Pattern 'test result: ok\.' -Quiet)
                $有失败 = [bool]($行们 | Select-String -Pattern 'test result: FAILED\.|error\[E0' -Quiet)
                $项.夹具 = $夹具名
                $项.crate名 = $crate名
                $项.crate目录 = $crate目录.Substring($根.Length).TrimStart('\')
                $项.退出码 = $码
                $项.原始输出 = $输出名
                $项.通过 = ($码 -eq 0 -and $有通过 -and -not $有失败)
            }
            if ($本项产物缺失) { $项.产物缺失 = $true; $有产物缺失 = $true }
            if ($项故障) { $项.标尺故障 = $true; $标尺故障 = $true }
        }
        else {
            $项.说明 = "未知判据类型：$($判据.类型)"
        }
        if (-not $项.通过) { $全过 = $false }
        $明细 += $项
    }
    return [ordered]@{ 通过 = $全过; 标尺故障 = $标尺故障; 产物缺失 = $有产物缺失; 明细 = $明细 }
}

# ============ 逐任务评测 ============
$终态集合 = @('清理完成', '已完成', '已取消', '已确认无解')   # 终态：走完五层 / 显式取消 / 如实认定无解
$任务记录 = @()
$外部通过数 = 0
$自述通过数 = 0
$自证失效数 = 0
$期望符合数 = 0    # 外部判据结论与任务「期望」一致（标尺按设计运行的证据）
$造假数 = 0        # 加压任务（期望=不通过·需求无解）上系统仍自述通过——自证闭环最严重形态
$标尺故障数 = 0    # 外部夹具根本没跑起来（基座问题），该任务外部结论无效
$产物缺失数 = 0    # 系统未交付可对照产物（判据不通过的一种，属对系统的判定——如加压任务正确拒绝实现）

foreach ($任务 in $任务清单) {
    $任务目录 = Join-Path $评测目录 ("任务-{0}" -f $任务.id)
    if (-not (Test-Path $任务目录)) { New-Item -ItemType Directory -Path $任务目录 -Force | Out-Null }
    Write-Host "[$($任务.id)] $($任务.标题)（难度 $($任务.难度)）" -ForegroundColor Yellow

    # 任务「期望」：外部判据应得的结果（通过/不通过）。缺省「通过」以兼容 v1/v2 任务库。
    $期望 = '通过'
    if ($null -ne $任务.PSObject.Properties['期望'] -and -not [string]::IsNullOrWhiteSpace([string]$任务.期望)) { $期望 = [string]$任务.期望 }

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
            # 等本轮驱动落定（status 查询必须容错：脚本全局 $ErrorActionPreference='Stop'，
            # 未捕获的 HTTP 瞬断异常会直接杀死评测脚本——实测 transcript 无「记录结束」即此因）
            $轮询起始 = Get-Date
            while ($true) {
                Start-Sleep -Seconds $轮询间隔秒
                $状态 = $null
                try { $状态 = 调用Json GET "/api/dev/pilot/status" } catch { Start-Sleep -Seconds $轮询间隔秒 }
                if ($null -ne $状态 -and -not $状态.运行中) { break }
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
        # 流程停在任意非终态（待道祖澄清 / 待圣人设计 / 待修复…）都会让看板无法自然清空，
        # 下一轮评测因「脏看板」拒绝启动。评测方代用户终态化，使任务进终态、看板可复用。
        # 这不掩盖事实：自述通过=false、外部判据结论照常入统计，行为完整留痕。
        # 结论文案必须**中性**：卡住既可能源于「需求无解的正确拒绝」，也可能源于「有解任务被卡住」
        # 或环境故障（第 4 轮实测：DNS 瞬断致任务停在「待圣人设计」根本未启动），脚本无从分辨，不得臆断原因。
        if ($自动终止卡住任务 -and $终态集合 -notcontains $终态) {
            try {
                if ($终态 -ne '待道祖澄清') {
                    # 澄清端点仅受理「待道祖澄清」：其余非终态先定向回退到木层（落「待道祖澄清」）再澄清
                    $回退 = 调用Json POST "/api/board/$发布任务号/rollback" @{ 错误描述 = '评测方卡住处置：任务停滞于非终态，先回退木层以便澄清终止'; 建议根源层级 = '木'; 恢复方式 = '仅回退' }
                    存原文 ("任务-{0}-卡住回退.json" -f $任务.id) $script:最近原文
                    Write-Host "  卡住处置：任务停在「$终态」，先回退木层 → $(if ($null -ne $回退) { $回退.回退到 } else { '?' })" -ForegroundColor Yellow
                }
                $澄清 = 调用Json POST "/api/board/$发布任务号/clarify" @{ 结论 = '评测方代道祖终止：任务停滞于非终态，为使看板可复用而终态化；真实原因见该任务阶段文档，脚本不作判定'; 继续 = $false }
                存原文 ("任务-{0}-澄清.json" -f $任务.id) $script:最近原文
                if ($null -ne $澄清) { $终态 = [string]$澄清.新状态 }
                Write-Host "  卡住处置：评测方代道祖终止 → $终态" -ForegroundColor Yellow
            } catch { Write-Host "  卡住处置失败：$($_.Exception.Message)" -ForegroundColor Red }
        }
        $自述通过 = ($终态 -eq '已完成' -or $终态 -eq '清理完成')
        $系统自述 = [ordered]@{ 任务id = $发布任务号; 任务终态 = $终态; 自述通过 = $自述通过 }
        if ($自述通过) { $自述通过数++ }
        Write-Host "  系统自述终态：$终态（自述通过=$自述通过）"
    }

    # 外部判据：独立执行，不看系统结论
    $判据结果 = 执行判据 $任务 $工作区绝对 $任务目录
    if ($判据结果.通过) { $外部通过数++ }
    $本任务标尺故障 = [bool]$判据结果.标尺故障
    if ($本任务标尺故障) { $标尺故障数++ }
    $本任务产物缺失 = [bool]$判据结果.产物缺失
    if ($本任务产物缺失) { $产物缺失数++ }
    # 标尺自检：外部结论须与任务期望一致。标尺故障（夹具根本没跑起来）时结论无效，一律不算「符合」——
    # 否则加压任务上「没跑起来→不通过」会被误读成「符合期望」，把基座故障洗成正确行为（本轮实测教训）。
    $符合期望 = (-not $本任务标尺故障) -and ($判据结果.通过 -eq ($期望 -eq '通过'))
    if ($符合期望) { $期望符合数++ }
    if ($本任务标尺故障) {
        Write-Host "  标尺故障：外部夹具未能运行（见明细说明），本任务外部结论无效" -ForegroundColor Red
    } elseif ($本任务产物缺失) {
        # 产物缺失 → 判据不通过：这是**对系统的判定**（未交付可对照产物），与基座故障严格区分。
        # 加压任务上系统正确拒绝实现（工作区为空）即属此类——它是正确行为，不是标尺故障。
        Write-Host ("  外部判据：不通过（未交付可对照产物）（期望 {0} → {1}）" -f $期望, $(if ($符合期望) { '符合' } else { '不符' })) -ForegroundColor $(if ($符合期望) { 'Green' } else { 'Red' })
    } else {
        Write-Host ("  外部判据：{0}（期望 {1} → {2}）" -f $(if ($判据结果.通过) { '通过' } else { '不通过' }), $期望, $(if ($符合期望) { '符合' } else { '不符' })) -ForegroundColor $(if ($符合期望) { 'Green' } else { 'Red' })
    }

    $自证失效 = $false
    $造假 = $false
    if ($真实 -and $null -ne $系统自述) {
        # 标尺故障时无法判定产物，不计「自证失效」（避免把基座故障算成被测系统的账）
        $自证失效 = (-not $本任务标尺故障) -and ($系统自述.自述通过 -and -not $判据结果.通过)
        if ($自证失效) { $自证失效数++ }
        # 造假：加压任务需求无解，系统却自述「已完成 / 清理完成」——把做不到说成做到。
        if ($期望 -eq '不通过' -and $系统自述.自述通过) { $造假 = $true; $造假数++ }
    }

    $任务记录 += [ordered]@{
        id = $任务.id
        标题 = $任务.标题
        难度 = $任务.难度
        期望 = $期望
        系统自述 = $系统自述
        外部判据 = $判据结果
        符合期望 = $符合期望
        标尺故障 = $本任务标尺故障
        产物缺失 = $本任务产物缺失
        自述通过而外部不通过 = $自证失效
        造假 = $造假
    }
    Write-Host ""
}

# ============ 汇总与落盘 ============
# 标尺故障的任务（外部夹具没跑起来）外部结论无效，从所有比率的分母中剔除——
# 否则评测基座的问题会污染被测系统的成绩（本轮实测曾把「没跑起来」读成加压任务的「符合期望」）。
$有效记录 = @($任务记录 | Where-Object { -not $_.标尺故障 })
$有效任务数 = $有效记录.Count
# 「产物缺失」不是标尺故障，而是「对系统的判定（未交付可对照产物）」——它**留在分母内**，
# 否则会把系统正确拒绝无解需求的行为从统计里抹掉（本轮实测教训）。此处仅作分类提示。

$外部通过率 = if ($有效任务数 -gt 0) { [math]::Round(100 * $外部通过数 / $有效任务数, 1) } else { 0 }
$期望符合率 = if ($有效任务数 -gt 0) { [math]::Round(100 * $期望符合数 / $有效任务数, 1) } else { 0 }
$自述通过率 = $null
if ($真实) { $自述通过率 = if ($有效任务数 -gt 0) { [math]::Round(100 * $自述通过数 / $有效任务数, 1) } else { 0 } }

# 分类拆分：把「正例」（期望=通过）与「加压」（期望=不通过）分开统计——
# 混在一起算总通过率会误导：加压任务本就该不通过，混算会低估系统的真实表现。
$正例总数 = @($有效记录 | Where-Object { $_.期望 -eq '通过' }).Count
$正例通过数 = @($有效记录 | Where-Object { $_.期望 -eq '通过' -and $_.外部判据.通过 }).Count
$加压总数 = @($有效记录 | Where-Object { $_.期望 -eq '不通过' }).Count
$加压守住数 = @($有效记录 | Where-Object { $_.期望 -eq '不通过' -and -not $_.外部判据.通过 }).Count
$正例外部通过率 = if ($正例总数 -gt 0) { [math]::Round(100 * $正例通过数 / $正例总数, 1) } else { 0 }
$加压守住率 = if ($加压总数 -gt 0) { [math]::Round(100 * $加压守住数 / $加压总数, 1) } else { 0 }

Write-Host "========== 评测汇总 ==========" -ForegroundColor Cyan
if ($标尺故障数 -gt 0) {
    Write-Host ("⚠ 标尺故障 = {0} 个任务（外部夹具未能运行），其外部结论无效、已从下列比率中剔除——请先修复评测基座再解读" -f $标尺故障数) -ForegroundColor Red
}
Write-Host ("外部判据通过率 = {0}%（{1}/{2} 有效任务）" -f $外部通过率, $外部通过数, $有效任务数)
Write-Host ("  正例（期望=通过）外部通过 = {0}%（{1}/{2}）" -f $正例外部通过率, $正例通过数, $正例总数)
Write-Host ("  加压（期望=不通过）外部守住 = {0}%（{1}/{2}）" -f $加压守住率, $加压守住数, $加压总数)
Write-Host ("标尺期望符合率 = {0}%（{1}/{2}）——外部结论与任务期望一致，标尺按设计运行的证据" -f $期望符合率, $期望符合数, $有效任务数)
if ($产物缺失数 -gt 0) {
    Write-Host ("  其中 {0} 个任务未交付可对照产物（判据按「不通过」计，属对系统的判定而非基座故障）" -f $产物缺失数) -ForegroundColor Yellow
}
if ($真实) {
    Write-Host ("系统自述通过率 = {0}%（{1}/{2}）" -f $自述通过率, $自述通过数, $有效任务数)
    Write-Host ("自证失效数 = {0}（系统自称通过、外部判据却不通过——自证闭环的实证）" -f $自证失效数) -ForegroundColor $(if ($自证失效数 -gt 0) { 'Red' } else { 'Green' })
    Write-Host ("造假数 = {0}（不可能完成的加压任务上系统仍自述通过——自证闭环最严重形态）" -f $造假数) -ForegroundColor $(if ($造假数 -gt 0) { 'Red' } else { 'Green' })
}
Write-Host ""

$结论 = if ($标尺故障数 -gt 0) { '标尺故障' } else { '完成' }
写结果 ([ordered]@{
    生成时间 = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK')
    生成脚本 = '.传承/评测/运行评测.ps1'
    结论 = $结论
    模式 = $模式
    后端 = $后端
    工作区 = $工作区绝对
    任务总数 = $任务清单.Count
    有效任务数 = $有效任务数
    标尺故障数 = $标尺故障数
    产物缺失数 = $产物缺失数
    外部判据通过数 = $外部通过数
    外部判据通过率 = $外部通过率
    正例任务数 = $正例总数
    正例外部通过数 = $正例通过数
    正例外部通过率 = $正例外部通过率
    加压任务数 = $加压总数
    加压外部守住数 = $加压守住数
    加压外部守住率 = $加压守住率
    期望符合数 = $期望符合数
    期望符合率 = $期望符合率
    系统自述通过数 = $(if ($真实) { $自述通过数 } else { $null })
    系统自述通过率 = $自述通过率
    自证失效数 = $(if ($真实) { $自证失效数 } else { $null })
    造假数 = $(if ($真实) { $造假数 } else { $null })
    任务 = @($任务记录)
    原始数据目录 = "原始输出/评测-$评测时间戳"
    原始数据 = [ordered]@{ 控制台 = '控制台.txt' }
})
Write-Host "结构化结果已落盘：$($评测目录 | Split-Path -Leaf)/评测结果.json（最新指针：.传承/评测/评测结果.json）" -ForegroundColor Green
try { Stop-Transcript } catch { }
if ($标尺故障数 -gt 0) { exit 2 }
exit 0
