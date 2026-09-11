# 端到端基准：真实 LLM 全链脚本化（发布 → 设计 → 实现 → 验收 → 终审 → 清理）
# 输出三指标：无人干预率 / 测试通过率 / 返工率
#
# 用法：
#   .\端到端基准.ps1                          # 默认任务，后端 http://localhost:8321
#   .\端到端基准.ps1 -地址 http://127.0.0.1:8321 -任务 "写一个冒泡排序模块" "写一个哈希表模块"
#
# 前置：后端已启动（.\启动.ps1）且 LLM 已接入（default.toml [llm] 段或环境变量密钥）。
param(
    [string]$地址 = "http://localhost:8321",
    [string[]]$任务清单 = @(),
    [int]$轮询超时秒 = 900,
    [int]$轮询间隔秒 = 2
)

$ErrorActionPreference = "Stop"

# 前置硬校验：本脚本依赖 PS7 的字节级 UTF-8 解码（Invoke-WebRequest 响应的 Content 为 byte[]）。
# PS5.1 下响应文本会按本地代码页误解码，导致「驱动台就绪」判断失真（误报未装配），
# 故此处直接拒绝运行并明确声明须用 pwsh 7+（而非仅告警后继续跑、崩在解码处）。
if ($PSVersionTable.PSVersion.Major -lt 7) {
    Write-Host "✗ 检测到 PowerShell $($PSVersionTable.PSVersion.ToString())（低于 7），本脚本需 pwsh 7+。" -ForegroundColor Red
    Write-Host "  请改用：pwsh -ExecutionPolicy Bypass -File .\.传承\门禁\端到端基准.ps1" -ForegroundColor Yellow
    exit 1
}

# 默认任务清单（覆盖五层协作典型需求）
if ($任务清单.Count -eq 0) {
    $任务清单 = @(
        "写一个返回斐波那契数列第 n 项的模块，附带单元测试"
    )
}

function 调用Json($方法, $路径, $体 = $null) {
    # 统一取响应原始字节并按 UTF-8 解码（PS7 下 Invoke-WebRequest 的 Content 即 byte[]），
    # 规避中文键/值按本地代码页误解码导致「驱动台就绪」判断失真。入口已硬校验 pwsh 7+；
    # 下方仍做多形态兜底（Content 字节数组 → RawContentStream → 字符串强转），
    # 避免不同 PS 包装对象属性差异导致解码处直接抛异常。
    if ($null -eq $体) {
        $响应 = Invoke-WebRequest -Uri "$地址$路径" -Method $方法 -UseBasicParsing
    } else {
        $响应 = Invoke-WebRequest -Uri "$地址$路径" -Method $方法 -Body ($体 | ConvertTo-Json -Compress) -ContentType "application/json; charset=utf-8" -UseBasicParsing
    }
    $原始字节 = $null
    if ($响应.Content -is [byte[]]) {
        $原始字节 = $响应.Content
    } elseif ($null -ne $响应.PSObject.Properties['RawContentStream'] -and $null -ne $响应.RawContentStream) {
        $原始字节 = $响应.RawContentStream.ToArray()
    }
    $文本 = if ($null -ne $原始字节) { [System.Text.Encoding]::UTF8.GetString($原始字节) } else { [string]$响应.Content }
    if ([string]::IsNullOrWhiteSpace($文本)) { return $null }
    $文本 | ConvertFrom-Json
}

# ============ 前置自检 ============
Write-Host "========== 端到端基准 · 真实 LLM 全链 ==========" -ForegroundColor Cyan
Write-Host "后端地址：$地址"
try {
    $状态 = 调用Json GET "/api/dev/pilot/status"
    if (-not $状态.就绪) {
        Write-Host "✗ 看板驱动台未装配（五层驱动器未就绪），请确认启动模块装配。请勿运行。" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ 无法连接后端 $地址，请先运行 .\启动.ps1 并接入 LLM。请勿运行。" -ForegroundColor Red
    exit 1
}
Write-Host "驱动台就绪：是" -ForegroundColor Green
Write-Host ""

# ============ 执行任务 ============
$发布任务数 = 0
foreach ($任务描述 in $任务清单) {
    $发布任务数++
    $标题 = if ($任务描述.Length -gt 24) { $任务描述.Substring(0, 24) } else { $任务描述 }
    Write-Host "[任务 $发布任务数/$($任务清单.Count)] 发布：$标题" -ForegroundColor Yellow

    $发布 = 调用Json POST "/api/board" @{ title = $标题; description = $任务描述 }
    Write-Host "  任务 #$($发布.id) 已发布，开始自主驱动…"

    # 发布自动仅推进第一层（圣人设计）；等待发布自动线程落定（运行中=false）后再显式 drain
    # 走完五层。避免「发布线程占用 → drain 409 → 脚本中断」的时序缺陷。
    $发布线程落定 = $false
    $状态接口失败 = $false
    $已等待秒 = 0
    for ($已等待秒 = 1; $已等待秒 -le 60; $已等待秒++) {
        Start-Sleep -Seconds 1
        try {
            $状态 = 调用Json GET "/api/dev/pilot/status"
            if (-not $状态.运行中) { $发布线程落定 = $true; break }
        } catch {
            # 状态接口瞬时失败不中断：交由 drain 重试兜底；记录以如实提示（不谎报「60 秒未落定」）
            $状态接口失败 = $true
            break
        }
    }
    if (-not $发布线程落定) {
        if ($状态接口失败) {
            Write-Host "  状态接口瞬时失败（已等待 $已等待秒 秒），继续尝试 drain（drain 自身带重试）" -ForegroundColor Yellow
        } else {
            Write-Host "  发布自动线程 60 秒仍未落定，继续尝试 drain（drain 自身带重试）" -ForegroundColor Yellow
        }
    }

    # drain 带重试（3 次，间隔轮询）：每次失败告警但不中断脚本
    $受理成功 = $false
    for ($重试 = 1; $重试 -le 3; $重试++) {
        try {
            调用Json POST "/api/dev/pilot/drain" @{ 上限 = 60 } | Out-Null
            $受理成功 = $true
            break
        } catch {
            Write-Host "  drain 第 $重试/3 次未受理（驱动线程可能占用），等待后重试…" -ForegroundColor Yellow
            Start-Sleep -Seconds ($轮询间隔秒 * 2)
        }
    }
    if (-not $受理成功) {
        Write-Host "  ✗ drain 3 次仍失败，请检查后端日志后重跑该任务。任务 #$($发布.id) 仍保留在看板。" -ForegroundColor Red
        continue
    }

    # 轮询到空闲
    $轮询起始 = Get-Date
    $轮询次数 = 0
    while ($true) {
        Start-Sleep -Seconds $轮询间隔秒
        $轮询次数++
        $状态 = 调用Json GET "/api/dev/pilot/status"
        if (-not $状态.运行中) {
            $最近结果 = if ($状态.最近结果) { $状态.最近结果 } else { '—' }
            Write-Host "  驱动结束（最近结果：$最近结果）" -ForegroundColor Green
            break
        }
        if (((Get-Date) - $轮询起始).TotalSeconds -gt $轮询超时秒) {
            Write-Host "  驱动轮询超时（$轮询超时秒 秒），跳过等待" -ForegroundColor Red
            break
        }
        if ($轮询次数 % 15 -eq 0) {
            Write-Host "  驱动中…（已 $([int]((Get-Date) - $轮询起始).TotalSeconds) 秒）"
        }
    }
    # 若 drain 受理的是多轮，等待其落定（驱动线程内部连续推进，最后清空看板后才空闲）
    Start-Sleep -Seconds 2
}

# ============ 收集会话并计算三指标 ============
Write-Host ""
Write-Host "========== 指标统计 ==========" -ForegroundColor Cyan

$清单 = 调用Json GET "/api/dev/sessions"
$会话们 = @($清单.会话)
if ($会话们.Count -eq 0) {
    Write-Host "✗ 未收集到任何驱动会话，无法输出指标（请确认任务已成功发布并驱动）。请勿运行。" -ForegroundColor Red
    exit 1
}

# 任务终态定义：真正「无人工干预直达完成」须走完 木需求→火设计→土实现→金验收→水清理 五层。
# 仅「清理完成」（水层终点）与「已完成」（兼容终态）计为完成；中间态（待道祖终审/待清理/清理中）
# 尚未走完五层，不得计入完成，否则无人干预率虚高。状态枚举无「已交付」，故不列。
$完成状态集合 = @("已完成", "清理完成")
$失败状态集合 = @("待修复")

$总会话 = $会话们.Count
$无人干预 = 0        # 任务终态 ∈ 完成状态集合（五层走完，无人工介入）
$失败会话 = 0        # 任务终态 ∈ 失败状态集合
$测试通过会话 = 0    # 工具结果含 cargo test 通过证据
$有测试动作会话 = 0  # 工具调用运行命令含 test（实现阶段跑了测试）
$返工会话 = 0        # 事件含「待修复」（准圣打回）

# 收集本次驱动涉及的任务 id（会话任务id列表）
$涉及任务id = @()
foreach ($会话 in $会话们) {
    foreach ($tid in @($会话.任务id列表)) {
        if ($涉及任务id -notcontains $tid) { $涉及任务id += $tid }
    }
}
# 读取看板终态（任务最新状态）
$看板全部 = @()
if ($涉及任务id.Count -gt 0) {
    try { $看板全部 = @(调用Json GET "/api/board") } catch { $看板全部 = @() }
}
$任务终态 = @{}
foreach ($任务 in $看板全部) {
    $任务终态[$任务.id] = $任务.status
}

foreach ($会话 in $会话们) {
    # 会话级终态判定：一个会话可关联多个任务，须先按会话聚合再计数，
    # 否则同一会话多任务会重复累加（曾致「其他」为负、指标失真）。
    $本会话完成 = $false
    $本会话失败 = $false
    foreach ($tid in @($会话.任务id列表)) {
        if ($任务终态.ContainsKey($tid) -and $完成状态集合 -contains $任务终态[$tid]) { $本会话完成 = $true }
        if ($任务终态.ContainsKey($tid) -and $失败状态集合 -contains $任务终态[$tid]) { $本会话失败 = $true }
    }
    if ($本会话完成) { $无人干预++ }
    if ($本会话失败) { $失败会话++ }

    $详情 = 调用Json GET "/api/dev/sessions/$($会话.会话id)"
    $事件们 = @($详情.事件)

    $本会话有测试动作 = $false
    $本会话测试失败 = $false   # 实现阶段 cargo test 曾失败（显式返工信号）
    $本会话返工 = $false       # 每会话重置：准圣打回「待修复」或隐式修复（防跨会话泄漏）
    $本会话测试动作序列 = @()

    foreach ($事件 in $事件们) {
        $内容 = [string]$事件.内容
        $事件类型 = [string]$事件.类型
        $工具名 = [string]$事件.工具名
        # 测试动作：运行命令工具且命令含 cargo test / cargo test -p ...
        if ($事件类型 -eq "工具调用" -and $工具名 -eq "运行命令" -and $内容 -match "cargo\s+test") {
            $本会话有测试动作 = $true
            $本会话测试动作序列 += $内容
        }
        # 显式打回（准圣「待修复」事件）
        if ($内容 -match "待修复") { $本会话返工 = $true }
        # 测试失败证据：命令失败退出码（1/2/…/101，排除 0=成功）/ 编译错误 / 测试失败 / 断言 panic。
        # 不依赖完整 `test result: ok.` 行（工具结果可能被服务端截断），只认失败前缀。
        if ($事件类型 -eq "工具结果" -and $内容 -match "命令失败（退出码 [1-9][0-9]*|error\[E0|test result: FAILED|panicked") {
            $本会话测试失败 = $true
        }
    }

    # 测试通过判定（新口径，绕开工具结果截断导致 test result: ok. 行丢失的假阴性）：
    # 有测试动作 且 关联任务终态 ∈ 完成集合（五层全链走完 = 实现阶段测试最终通过）
    $本会话测试通过 = $本会话有测试动作 -and $本会话完成

    # 返工判定（新口径）：显式打回（待修复）算返工；隐式修复——测试失败后同会话又出现后续
    # 测试动作（失败→改→重测），或失败后任务仍走完五层——也算返工
    $隐式修复 = $本会话测试失败 -and ($本会话测试动作序列.Count -ge 2 -or $本会话完成)
    if ($隐式修复) { $本会话返工 = $true }

    if ($本会话有测试动作) { $有测试动作会话++ }
    if ($本会话测试通过) { $测试通过会话++ }
    if ($本会话返工) { $返工会话++ }
}

# 计算三指标（百分比，保留 1 位小数）
$无人干预率 = if ($总会话 -gt 0) { [math]::Round(100 * $无人干预 / $总会话, 1) } else { 0 }
$返工率 = if ($总会话 -gt 0) { [math]::Round(100 * $返工会话 / $总会话, 1) } else { 0 }
$测试通过率 = if ($有测试动作会话 -gt 0) { [math]::Round(100 * $测试通过会话 / $有测试动作会话, 1) } else { $null }

Write-Host ""
Write-Host "总会话数：$总会话（无干预直达完成 $无人干预 / 失败 $失败会话 / 其他 $($总会话 - $无人干预 - $失败会话)）"
Write-Host "任务终态：$((($任务终态.GetEnumerator() | ForEach-Object { "任务$($_.Key)=$($_.Value)" }) -join '、'))"
Write-Host ""
Write-Host "三指标：" -ForegroundColor Cyan
Write-Host ("  无人干预率 = {0}%（{1}/{2} 个驱动会话关联任务走完五层，全程无人工介入）" -f $无人干预率, $无人干预, $总会话)
if ($null -eq $测试通过率) {
    Write-Host "  测试通过率 = N/A（无会话在实现阶段运行过 cargo test）"
} else {
    Write-Host ("  测试通过率 = {0}%（{1}/{2} 有测试动作的会话产出通过测试）" -f $测试通过率, $测试通过会话, $有测试动作会话)
}
Write-Host ("  返工率 = {0}%（{1}/{2} 会话出现返工：准圣打回「待修复」或测试失败后自愈重测）" -f $返工率, $返工会话, $总会话)
Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
