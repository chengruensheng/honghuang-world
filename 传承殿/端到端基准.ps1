# 端到端基准：真实 LLM 全链脚本化（发布 → 设计 → 实现 → 验收 → 终审 → 清理）
# 输出三指标：无人干预率 / 测试通过率 / 返工率
#
# 用法：
#   .\端到端基准.ps1                          # 默认任务，后端 http://localhost:8321
#   .\端到端基准.ps1 -地址 http://127.0.0.1:8321 -任务 "写一个冒泡排序模块" "写一个哈希表模块"
#
# 前置：后端已启动（.\启动.ps1）且 LLM 已接入（default.toml [llm] 或前端「＋ 接入供应商」）。
param(
    [string]$地址 = "http://localhost:8321",
    [string[]]$任务清单 = @(),
    [int]$轮询超时秒 = 900,
    [int]$轮询间隔秒 = 2
)

$ErrorActionPreference = "Stop"

# 默认任务清单（覆盖五层协作典型需求）
if ($任务清单.Count -eq 0) {
    $任务清单 = @(
        "写一个返回斐波那契数列第 n 项的模块，附带单元测试"
    )
}

function 调用Json($方法, $路径, $体 = $null) {
    if ($null -eq $体) {
        Invoke-RestMethod -Uri "$地址$路径" -Method $方法
    } else {
        Invoke-RestMethod -Uri "$地址$路径" -Method $方法 -Body ($体 | ConvertTo-Json -Compress) -ContentType "application/json; charset=utf-8"
    }
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
    Write-Host "  任务 #$($发布.id) 已发布，五层协作开始自主流转…"

    # 发布已自动驱动到空闲（受理编排内启动），此处轮询到空闲
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

$总会话 = $会话们.Count
$无人干预 = 0        # 状态=已完成（五层走完，无人工介入）
$失败会话 = 0        # 状态=失败
$测试通过会话 = 0    # 工具结果含 cargo test 通过证据
$有测试动作会话 = 0  # 工具调用运行命令含 test（实现阶段跑了测试）
$返工会话 = 0        # 事件含「待修复」（准圣打回）

foreach ($会话 in $会话们) {
    if ($会话.状态 -eq "已完成") { $无人干预++ }
    if ($会话.状态 -eq "失败") { $失败会话++ }

    $详情 = 调用Json GET "/api/dev/sessions/$($会话.会话id)"
    $事件们 = @($详情.事件)

    $本会话测试通过 = $false
    $本会话有测试动作 = $false
    $本会话返工 = $false

    foreach ($事件 in $事件们) {
        $内容 = [string]$事件.内容
        if ($内容 -match "待修复") { $本会话返工 = $true }
        if ($事件.类型 -eq "工具调用" -and $事件.工具名 -eq "运行命令" -and $内容 -match "test") {
            $本会话有测试动作 = $true
        }
        if ($事件.类型 -eq "工具结果" -and $内容 -match "test result:\s*ok|passed|全绿") {
            $本会话测试通过 = $true
        }
    }

    if ($本会话有测试动作) { $有测试动作会话++ }
    if ($本会话测试通过) { $测试通过会话++ }
    if ($本会话返工) { $返工会话++ }
}

# 计算三指标（百分比，保留 1 位小数）
$无人干预率 = if ($总会话 -gt 0) { [math]::Round(100 * $无人干预 / $总会话, 1) } else { 0 }
$返工率 = if ($总会话 -gt 0) { [math]::Round(100 * $返工会话 / $总会话, 1) } else { 0 }
$测试通过率 = if ($有测试动作会话 -gt 0) { [math]::Round(100 * $测试通过会话 / $有测试动作会话, 1) } else { $null }

Write-Host ""
Write-Host "总会话数：$总会话（已完成 $无人干预 / 失败 $失败会话 / 其他 $($总会话 - $无人干预 - $失败会话)）"
Write-Host ""
Write-Host "三指标：" -ForegroundColor Cyan
Write-Host ("  无人干预率 = {0}%（{1}/{2} 会话全程无人工介入直达完成）" -f $无人干预率, $无人干预, $总会话)
if ($null -eq $测试通过率) {
    Write-Host "  测试通过率 = N/A（无会话在实现阶段运行过 cargo test）"
} else {
    Write-Host ("  测试通过率 = {0}%（{1}/{2} 有测试动作的会话产出通过测试）" -f $测试通过率, $测试通过会话, $有测试动作会话)
}
Write-Host ("  返工率 = {0}%（{1}/{2} 会话出现准圣打回「待修复」）" -f $返工率, $返工会话, $总会话)
Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
