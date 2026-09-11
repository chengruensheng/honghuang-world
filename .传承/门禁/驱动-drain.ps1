# 项目根：本脚本位于 .传承/门禁/，向上回溯 2 层即项目根
$根 = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$日志 = Join-Path $根 '道果\drain-删除防护验证.log'
"drain 开始 $(Get-Date -Format 'HH:mm:ss')" | Out-File $日志 -Encoding utf8
try {
    $r = Invoke-RestMethod -Uri 'http://localhost:8321/api/dev/pilot/drain' -Method Post -Body '{}' -ContentType 'application/json' -UseBasicParsing -TimeoutSec 3600
    ($r | ConvertTo-Json -Depth 5) | Out-File $日志 -Append -Encoding utf8
} catch {
    "drain 异常: $($_.Exception.Message)" | Out-File $日志 -Append -Encoding utf8
}
"drain 结束 $(Get-Date -Format 'HH:mm:ss')" | Out-File $日志 -Append -Encoding utf8
