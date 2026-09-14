# 校验unwrap滥用.ps1 —— 生产代码 unwrap 数量是否 <= 10
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_unwrap.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".", [int]$Threshold = 10)
$name = "unwrap数量"
$count = 0
$locations = @()
# 排除：测试/规格/证道/旧规则区/target/工作区沙箱/评测沙箱（探索产物，非本体源码）/.传承（夹具与评测辅助设施）
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区|\\评测沙箱|\\.传承" } | ForEach-Object {
    $lines = Get-Content $_.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "\.unwrap\(\)") {
            $count++
            $locations += "$($_.FullName):$($i+1)"
        }
    }
}
$jsonLocations = if ($locations.Count -gt 0) { $locations | ConvertTo-Json } else { "[]" }
if ($count -le $Threshold) {
    Write-Output ('{"name":"unwrap数量","passed":true,"evidence":"生产代码 unwrap 数量 ' + $count + ' <= ' + $Threshold + '","details":' + $jsonLocations + '}')
    exit 0
} else {
    Write-Output ('{"name":"unwrap数量","passed":false,"evidence":"生产代码 unwrap 数量 ' + $count + ' > ' + $Threshold + '","details":' + $jsonLocations + '}')
    exit 1
}
