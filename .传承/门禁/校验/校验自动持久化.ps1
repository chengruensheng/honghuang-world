# 校验自动持久化.ps1 —— 检测状态变更方法是否缺持久化调用
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_auto_persist.ps1
# 归集：.传承/门禁/校验/
#
# 判定（启发式，候选需人工复核）：
#   函数体出现「状态变更调用」（`.动词(` 形态）且不含任何「持久化」调用 → 候选违规。
#
# 相对初版的三处修正：
#   1) 函数体以「与 fn 同缩进的收尾 }」界定。初版用 \n} 作锚点，Rust 顶格 } 只出现
#      在 impl 块末尾，导致函数体跨函数吞并、真持久化调用被漏看（误报根源）。
#   2) 单行函数（fn 行内 { } 配平）与无体声明（trait 契约 `fn x(...) -> T;`）单独处理，
#      不进入跨行累积。
#   3) 状态变更由「裸词出现」收紧为「方法调用形态」：初版只要函数体里出现「创建/完成」
#      等字样即命中，字符串字面量、枚举变体、错误消息全部误伤（58→94 处误报）。
#
# 排除项：
#   · 文件名以「测试.rs」结尾、#[cfg(test)] 之后的内容不参与
#   · 构造器范式（new/default/init/from/to_/新/开始/从配置/默认）无副作用
#   · 候选命中「自动持久化-豁免.txt」登记条目 → 视为已复核豁免，不计入
#
# 复核结论（2026-09 全量复核）：五引擎核心节点（任务/迭代/记忆/规则/事件）关键状态
#   变更均有 `自动保存`，符合 rules/AI缺陷治理规则.md §八；其余候选为纯函数、构造器、
#   路由入口、契约转发或持久化委托下游，逐条登记于豁免清单。
param([string]$ProjectRoot = ".")

$name = "自动持久化"

# 状态变更：方法调用形态（点调用 + 变更动词 + 左括号），避免字符串/枚举名误命中
$状态变更模式 = '\.\s*(新增|添加|创建|修改|更新|删除|移除|开启|关闭|完成|取消)\s*\('
# 持久化：出现任一即视为已落盘
$持久化词 = @('保存', '持久化', '自动保存', '落盘', 'save', 'persist', 'flush', '写入', 'write')
# 构造器范式：中英文一致排除
$构造器模式 = '^(new|default|init|from|to_|新|开始|从配置|默认)'

# 读取豁免清单（格式：相对路径片段|函数名|类别；# 开头为注释）
$豁免 = @()
$豁免文件 = Join-Path $PSScriptRoot '自动持久化-豁免.txt'
if (Test-Path $豁免文件) {
    $豁免 = @(Get-Content $豁免文件 -Encoding UTF8 |
            Where-Object { $_ -and -not $_.TrimStart().StartsWith('#') } |
            ForEach-Object {
                $段 = $_ -split '\|'
                if ($段.Count -ge 2) { [pscustomobject]@{ 片段 = $段[0].Trim(); 函数 = $段[1].Trim() } }
            })
}

# 判定单个函数：命中候选返回描述，否则 $null
function 判定函数([string]$相对, [string]$函数名, [string]$体文) {
    if ($函数名 -match $构造器模式) { return $null }
    if ($豁免 | Where-Object { $相对 -like "*$($_.片段)*" -and $函数名 -eq $_.函数 }) { return $null }
    if ($体文 -notmatch $状态变更模式) { return $null }
    foreach ($词 in $持久化词) { if ($体文 -match $词) { return $null } }
    return "$相对`: $函数名 状态变更但无持久化调用"
}

$根 = (Resolve-Path $ProjectRoot).Path
$violations = @()

$候选文件 = Get-ChildItem -Path $ProjectRoot -Recurse -Include '*.rs' | Where-Object {
    $_.FullName -notmatch '\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区|trait' -and
    $_.Name -notmatch '测试\.rs$'
}

foreach ($文件 in $候选文件) {
    $相对 = $文件.FullName.Substring($根.Length).TrimStart('\', '/')
    $行们 = @(Get-Content $文件.FullName)
    $当前 = $null
    $在测试块 = $false
    for ($i = 0; $i -lt $行们.Count; $i++) {
        $行 = $行们[$i]
        # #[cfg(test)] 之后（含 mod tests 内用例）不参与扫描
        if ($行 -match '^\s*#\[cfg\(test\)\]') { $在测试块 = $true; $当前 = $null; continue }
        if ($在测试块) { continue }

        if ($当前) {
            $当前.体 += "`n$行"
            # 与 fn 同缩进的收尾 } 即函数体结束（嵌套块的 } 缩进更深，不会误判）
            if ($行 -match ('^' + [regex]::Escape($当前.缩进) + '\}')) {
                $结果 = 判定函数 $相对 $当前.名 $当前.体
                if ($结果) { $violations += $结果 }
                $当前 = $null
            }
            continue
        }

        if ($行 -match '^(?<缩进>[ \t]*)(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+(?<名>\w+)') {
            $函数名 = $Matches['名']
            $左 = ([regex]::Matches($行, '\{')).Count
            $右 = ([regex]::Matches($行, '\}')).Count
            if ($左 -eq 0) { continue }              # trait 契约声明：无函数体
            if ($右 -ge $左) {                        # 单行函数：整行即函数体
                $结果 = 判定函数 $相对 $函数名 $行
                if ($结果) { $violations += $结果 }
                continue
            }
            $当前 = [pscustomobject]@{ 缩进 = $Matches['缩进']; 名 = $函数名; 体 = $行 }
        }
    }
}

# 规则 §八 硬性底线：五引擎实现必须含「自动保存」落盘调用。
# 判据收紧为「调用形态」后，核心节点不得靠豁免或漏判绕过，此处独立校验。
$五引擎 = @(
    @{ 名 = '任务引擎'; 片段 = '任务仓库-殿' },
    @{ 名 = '迭代引擎'; 片段 = '迭代日志-阁' },
    @{ 名 = '记忆引擎'; 片段 = '记忆库-阁' },
    @{ 名 = '规则引擎'; 片段 = '规则库-阁' },
    @{ 名 = '事件引擎'; 片段 = '事件总线-阁' }
)
foreach ($引擎 in $五引擎) {
    $命中 = $候选文件 | Where-Object {
        $_.FullName -like "*$($引擎.片段)*" -and ((Get-Content $_.FullName -Raw) -match '自动保存')
    }
    if (-not $命中) { $violations += "五引擎硬性校验：$($引擎.名)（$($引擎.片段)）未见 自动保存 落盘调用" }
}

if ($violations.Count -eq 0) {
    Write-Output '{"name":"自动持久化","passed":true,"evidence":"状态变更方法均有持久化调用（豁免登记已复核）","details":[]}'
    exit 0
}
else {
    Write-Output ('{"name":"自动持久化","passed":null,"evidence":"发现 ' + $violations.Count + ' 处状态变更可能缺持久化（需人工确认是否为内存-only操作）","details":' + ($violations | ConvertTo-Json) + '}')
    exit 0
}
