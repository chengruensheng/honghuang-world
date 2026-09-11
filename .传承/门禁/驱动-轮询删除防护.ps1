# 驱动-轮询删除防护.ps1 —— 轮询看板直至任务终态，验证「删除防护」事后核验
# 定位：进程编排-变体（规范动词「驱动-<模式>」），配合 驱动-drain.ps1 使用。
# 用法：后端运行中，轮询 /api/board 观察任务状态，直至 完成|回退|失败|归档。
# 日志：道果/轮询-删除防护验证.log（动态定位项目根，不写死盘符）。
$ErrorActionPreference = 'Stop'
# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$看板 = 'http://localhost:8321/api/board'
$进度 = Join-Path $根 '道果\轮询-删除防护验证.log'
"轮询开始 $(Get-Date -Format 'HH:mm:ss')" | Out-File $进度 -Encoding utf8
$上次 = ''
for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep -Seconds 30
    try {
        $任务 = Invoke-RestMethod -Uri $看板 -UseBasicParsing -TimeoutSec 10 | Select-Object -First 1
        $状态 = $任务.status
        if ($状态 -ne $上次) {
            "$(Get-Date -Format 'HH:mm:ss') 状态: $状态" | Out-File $进度 -Append -Encoding utf8
            $上次 = $状态
        }
        if ($状态 -match '完成|回退|失败|归档') {
            "$(Get-Date -Format 'HH:mm:ss') 终态: $状态" | Out-File $进度 -Append -Encoding utf8
            break
        }
    } catch {
        "$(Get-Date -Format 'HH:mm:ss') 查询异常: $($_.Exception.Message)" | Out-File $进度 -Append -Encoding utf8
    }
}
"轮询结束 $(Get-Date -Format 'HH:mm:ss')" | Out-File $进度 -Append -Encoding utf8
