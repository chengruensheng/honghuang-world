# 校验容器化误用.ps1 —— 检测可能未注册到容器的组件
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_container.ps1
# 归集：.传承/门禁/校验/
param([string]$ProjectRoot = ".")
$name = "容器注册"
$components = @()
$registrations = @()
Get-ChildItem -Path $ProjectRoot -Recurse -Include "*.rs" | Where-Object { $_.FullName -notmatch "\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区" } | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    if (-not $content) { return }
    $structMatches = [regex]::Matches($content, "(?:pub\s+)?struct\s+(\w+)\s*\{")
    foreach ($m in $structMatches) {
        $components += $m.Groups[1].Value
    }
    $regMatches = [regex]::Matches($content, "(?:register|注册|insert|add|push).*?(\w+)")
    foreach ($m in $regMatches) {
        $registrations += $m.Groups[1].Value
    }
}
$unregistered = $components | Where-Object { $registrations -notcontains $_ -and $_ -notmatch "Error|Config|State|Data|Payload|Signal|Event|Rule|Task|Iteration|Memory|Container|Engine|Bridge|Repository|Service|Handler|Listener|Builder|Factory|Impl|Trait" }
if (-not $unregistered -or $unregistered.Count -eq 0) {
    Write-Output '{"name":"容器注册","passed":true,"evidence":"所有组件均已注册","details":[]}'
    exit 0
} else {
    Write-Output ('{"name":"容器注册","passed":null,"evidence":"发现 ' + $unregistered.Count + ' 个可能未注册的组件（需人工确认是否需要注册）","details":' + ($unregistered | ConvertTo-Json) + '}')
    exit 0
}
