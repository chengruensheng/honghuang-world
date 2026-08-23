# 检查-AGENTS-一致性：AGENTS.md 列出的纪律点能在项目代码或门禁脚本中找到锚点。
# 依据：AGENTS.md（12 条现场纪律）+ 融合蓝图-设计稿.md §15.3 第一节「AGENTS.md 命令清单 = 残」。
#
# 用法：pwsh -File 检查-AGENTS-一致性.ps1
# 退出码：0 = 通过；1 = 某纪律点缺锚点

. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根
Set-Location $根

$缺失 = @()

# AGENTS.md 12 条纪律 → 必须能在代码或门禁脚本中找到对应锚点
$纪律点 = @(
    @{ 编号 = 1; 名称 = '执行任务前先读6 份设计文档'; 锚点搜索 = @('层级结构-设计.md', '多智能体架构设计.md', '项目心智模型-设计稿.md', '融合蓝图-设计稿.md', '上下文.md', '智能体.md') }
    @{ 编号 = 2; 名称 = '临时产物放临时文件夹'; 锚点搜索 = @('临时文件夹', '.gitignore') }
    @{ 编号 = 3; 名称 = '命名与注释严禁 AI 味'; 锚点搜索 = @('AI味', '中式') }
    @{ 编号 = 4; 名称 = '本项目为新项目，老项目仅参考'; 锚点搜索 = @('新项目', '老项目') }
    @{ 编号 = 5; 名称 = '技术日志走 rizhi_fu lib 根'; 锚点搜索 = @('rizhi_fu', '日志记录-府') }
    @{ 编号 = 6; 名称 = '新增代码必须带日志埋点'; 锚点搜索 = @('日志埋点', 'info!', 'warn!', 'error!', 'debug!') }
    @{ 编号 = 7; 名称 = '排查问题看真实内容（完整字段、提示词、记录）'; 锚点搜索 = @('真实内容', 'spill') }
    @{ 编号 = 8; 名称 = '先入稿、再落码（设计稿溯源）'; 锚点搜索 = @('入稿', '溯源', '活文档') }
    @{ 编号 = 9; 名称 = '临时文件夹只装临时产物；被进程锁的文件先结束进程再删'; 锚点搜索 = @('临时文件夹', '临时产物') }
    @{ 编号 = 10; 名称 = '根目录禁 src/、tests/、scripts/ 空目录'; 锚点搜索 = @('/src/', '/tests/', '/scripts/', '.gitignore') }
    @{ 编号 = 11; 名称 = '根目录禁 IDE 本地配置目录'; 锚点搜索 = @('.arts/', '.codeartsdoer/', '.vscode/', '.idea/') }
    @{ 编号 = 12; 名称 = '.env 只放实际读取的配置项'; 锚点搜索 = @('.env', 'WORLD_AI_TOKEN', 'LLM_API_KEY') }
)

Write-Host "AGENTS.md 纪律点锚点核查（共 $($纪律点.Count) 条）..."

foreach ($点 in $纪律点) {
    $找到 = $false
    foreach ($关键词 in $点.锚点搜索) {
        # 在项目 .rs / .ps1 / .toml / .gitignore / .md 中搜索关键词（限源码区，排除道果树）
        $匹配 = Get-ChildItem $根 -Recurse -File -Include '*.rs','*.ps1','*.toml','*.gitignore','*.md' -ErrorAction SilentlyContinue | Where-Object {
            $_.FullName -notmatch '\\道果树\\|\\\.上下文\\|\\临时文件夹\\|\\\.git\\|\\deps\\|\\incremental\\|\\target\\'
        } | Select-Object -First 50 | Where-Object {
            try { (Get-Content $_.FullName -Raw -Encoding UTF8) -match [regex]::Escape($关键词) }
            catch { $false }
        }
        if ($匹配.Count -gt 0) {
            $找到 = $true
            break
        }
    }
    if (-not $找到) {
        $缺失 += "第 $($点.编号) 条「$($点.名称)」"
        Write-Host "  [缺失] 第 $($点.编号) 条「$($点.名称)」"
    } else {
        Write-Host "  [通过] 第 $($点.编号) 条「$($点.名称)」"
    }
}

if ($缺失.Count -gt 0) {
    Write-Host ""
    Write-Host "[拦截] 共 $($缺失.Count) 条纪律点缺锚点："
    foreach ($项 in $缺失) {
        Write-Host "  - $项"
    }
    exit 1
}

Write-Host ""
Write-Host "[通过] 全部纪律点锚点齐全"
exit 0