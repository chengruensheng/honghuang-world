# 校验容器化误用.ps1 —— 检测五引擎是否注册到组件容器（规则 §十一 检测 10 硬性底线）
# 原文件：.codeartsdoer/skills/ai-defect-governance/scripts/detect_container.ps1
# 归集：.传承/门禁/校验/
#
# 判据（2026-09-12 复核后重写）：
#   初版为「全仓 struct 名 → 与 register/注册/insert/add/push 正则捕获比对」，
#   把请求响应 / DTO / 枚举 / 工具函数名统统算作「可能未注册的组件」，报 194 处，
#   与规则 §十一 检测 10 的通过标准（注册数 ≥ 6 = 信号总线 + 五引擎）毫无关系——
#   那 194 个名字里绝大多数根本不是组件，而是数据结构。判据从根上错位，故重写。
#
# 现判据（对齐规则硬性底线，逐要件可证伪）：
#   ① 五引擎各实现文件须有 `impl Component for`（注册名经 `引擎.name()` 取自该实现）；
#   ② 装配文件（联动装配-府/桥接模块.rs）须注册 ≥ 6 个组件：
#      信号总线直注册 1 + 五引擎经 `注册命名` 5。
#
# 范围说明：其它 `impl Component for X`（智能体 / 扫尾执行者 / LLM池 / 叙述器 / 执行器…）
#   按装配设计由「装配体类型化字段」直接持有，不经容器注册，不属本条检测范围。
param([string]$ProjectRoot = ".")

$name = "容器注册"

# 底线要件：注册名（Component::name() 返回值）→ 实现文件片段（用于证明组件真实存在）
$要件 = @(
    @{ 名 = '信号总线'; 片段 = '信号总线-府' },
    @{ 名 = '任务仓库'; 片段 = '任务仓库-殿' },
    @{ 名 = '迭代日志'; 片段 = '迭代日志-阁' },
    @{ 名 = '记忆库';   片段 = '记忆库-阁' },
    @{ 名 = '规则库';   片段 = '规则库-阁' },
    @{ 名 = '事件总线'; 片段 = '事件总线-阁' }
)

$violations = @()

# ① 各要件须有真实的组件实现（`impl Component for`）——否则注册名无处可来
foreach ($件 in $要件) {
    $命中 = Get-ChildItem -Path $ProjectRoot -Recurse -Include '*.rs' | Where-Object {
        $_.FullName -notmatch '\\test|\\spec|\\证道|\\.codeartsdoer|\\target|\\工作区' -and
        $_.FullName -like "*$($件.片段)*" -and
        ((Get-Content $_.FullName -Raw) -match 'impl\s+Component\s+for\s+\w+')
    }
    if (-not $命中) { $violations += "底线：$($件.名) 未见组件实现（impl Component for，期望落在 $($件.片段)）" }
}

# ② 装配文件须实际注册——注册数 ≥ 6（信号总线 1 + 五引擎 5）
$装配 = Get-ChildItem -Path $ProjectRoot -Recurse -Include '桥接模块.rs' |
    Where-Object { $_.FullName -like '*联动装配-府*' -or $_.FullName -like '*相生桥接-阁*' } |
    Select-Object -First 1
if (-not $装配) {
    $violations += '底线：未找到五行装配文件（联动装配-府 · 桥接模块.rs）'
} else {
    $文 = Get-Content $装配.FullName -Raw
    $直注册 = ([regex]::Matches($文, '容器\.注册\(')).Count
    $命名注册 = ([regex]::Matches($文, '容器\.注册命名\(')).Count
    $总数 = $直注册 + $命名注册
    if ($命名注册 -lt 5) {
        $violations += "底线：五引擎经 注册命名 注册仅 $命名注册 处（应 ≥ 5）"
    }
    if ($直注册 -lt 1) {
        $violations += '底线：信号总线未见直注册（容器.注册(..) 0 处）'
    }
    if ($总数 -lt 6) {
        $violations += "底线：容器注册数 $总数 < 6（应 = 信号总线 + 五引擎）"
    }
}

$说明 = '五引擎与信号总线均已注册到组件容器'
if ($装配) { $说明 += "（$($装配.Name)：直注册 $直注册 + 命名注册 $命名注册 = $总数）" }

if ($violations.Count -eq 0) {
    Write-Output ('{"name":"' + $name + '","passed":true,"evidence":"' + $说明 + '","details":[]}')
    exit 0
} else {
    Write-Output ('{"name":"' + $name + '","passed":false,"evidence":"发现 ' + $violations.Count + ' 项未达底线","details":' + ($violations | ConvertTo-Json) + '}')
    exit 1
}
