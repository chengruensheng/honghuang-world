# 校验生产含demo.ps1 —— 生产入口不得无条件调用演示/自检函数
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_demo_in_production.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "生产入口演示函数"
$violations = @()
$entryFiles = Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object {
    $_.Name -match "启动|main|lib|bootstrap|入口" -and
    $_.FullName -notmatch "\\target\\" -and
    $_.FullName -notmatch "\\证道\\" -and
    $_.FullName -notmatch "\\.codeartsdoer\\"
}
foreach ($f in $entryFiles) {
    $lines = Get-Content $f.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "(演示|示例|自检|冒烟|demo|example|smoke)" -and $lines[$i] -match "\.|\(|::") {
            $hasGuard = $false
            $start = [Math]::Max(0, $i - 5)
            for ($j = $start; $j -le $i; $j++) {
                if ($lines[$j] -match "if.*config|if.*cfg!|if.*debug|if.*self_test|if.*run_self|if.*self_test|if.*demo") {
                    $hasGuard = $true
                    break
                }
            }
            if (-not $hasGuard) {
                $violations += "$($f.Name):$($i+1): $($lines[$i].Trim())"
            }
        }
    }
}
if ($violations.Count -eq 0) {
    Write-Output '{"name":"生产入口演示函数","passed":true,"evidence":"生产入口无演示函数调用（或已有配置开关控制）","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"生产入口演示函数","passed":false,"evidence":"发现 ' + $violations.Count + ' 处生产入口无条件调用演示/自检函数","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
