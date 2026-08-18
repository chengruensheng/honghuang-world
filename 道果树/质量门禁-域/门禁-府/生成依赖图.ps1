# 生成依赖图：解析 .rs 跨府 use 引用，输出府间依赖 mermaid 图。
# 用法：pwsh -File 生成依赖图.ps1 [-输出 路径]
param([string]$输出 = "")

. (Join-Path $PSScriptRoot '公共-函数.ps1')
$根 = 找工作区根

# 府 lib 名 → 府路径 映射（用于把被引用方归到府）
$府表 = @{
    'shihai_fu' = '识海承载-府'; 'tianting_fu' = '天庭治理-府'; 'daoshu_fu' = '道术施展-府'
    'moxing_fu' = '模型连接-府'; 'rizhi_fu' = '日志记录-府'; 'peizhi_fu' = '配置管理-府'
    'mingling_fu' = '命令操作-府'; 'jiance_fu' = '观测探针-府'; 'zhengdao_fu' = '单元测试-府'
}
# 文件路径 → 所属府（按路径片段匹配）
$路径府映射 = @(
    @{ 片段 = '识海承载-府'; 名 = '识海承载-府' },
    @{ 片段 = '天庭治理-府'; 名 = '天庭治理-府' },
    @{ 片段 = '道术施展-府'; 名 = '道术施展-府' },
    @{ 片段 = '模型连接-府'; 名 = '模型连接-府' },
    @{ 片段 = '日志记录-府'; 名 = '日志记录-府' },
    @{ 片段 = '配置管理-府'; 名 = '配置管理-府' },
    @{ 片段 = '命令操作-府'; 名 = '命令操作-府' },
    @{ 片段 = '观测探针-府'; 名 = '观测探针-府' },
    @{ 片段 = '单元测试-府'; 名 = '单元测试-府' }
)

function 文件所属府([string]$路径) {
    foreach ($映射 in $路径府映射) {
        if ($路径 -match [regex]::Escape($映射.片段)) { return $映射.名 }
    }
    return $null
}

$边们 = @{}
$文件们 = 取源码文件 $根 '*.rs'
# 第三方/标准库 lib 名（不属本仓府，忽略）
$第三方 = @('std','core','alloc','serde','serde_json','thiserror','tracing','tracing_subscriber','ureq',
    'log','once_cell','lazy_static','time','flate2','base64','url','idna','percent_encoding','rustls',
    'webpki','ring','getrandom','untrusted','zeroize','subtle','crc32fast','simd_adler32','miniz_oxide',
    'smallvec','thread_local','cfg_if','nu_ansi_term','windows_sys','windows_link','sharded_slab',
    'pin_project_lite','memchr','itoa','ryu','zmij','num_conv','powerfmt','deranged','icu_properties',
    'icu_normalizer','icu_collections','utf8_iter','potential_utf','icu_provider','icu_locale_core',
    'tinystr','litemap','writeable','zerovec','zerotrie','yoke','stable_deref_trait','zerofrom',
    'equivalent','hashbrown','ahash','foldhash','regex','regex_automata','regex_syntax','aho_corasick',
    'env_logger','log','tempfile','rand','fastrand','getrandom','ppv_lite86','libc','winapi')
foreach ($文件 in $文件们) {
    $来源府 = 文件所属府 $文件.FullName
    if (-not $来源府) { continue }
    $内容 = Get-Content $文件.FullName -Raw -Encoding UTF8
    # 匹配 lib:: 路径限定引用（use 导入或全路径调用），排除 self/crate/super 与第三方
    foreach ($匹配 in [regex]::Matches($内容, '\b([a-z_]+)::')) {
        $被引用lib = $匹配.Groups[1].Value
        if ($被引用lib -eq 'crate' -or $被引用lib -eq 'self' -or $被引用lib -eq 'super') { continue }
        if ($第三方 -contains $被引用lib) { continue }
        if ($府表.ContainsKey($被引用lib)) {
            $目标府 = $府表[$被引用lib]
            if ($目标府 -ne $来源府) {
                $键 = "$来源府|$目标府"
                $边们[$键] = @{ 来源 = $来源府; 目标 = $目标府 }
            }
        }
    }
}

# 输出 mermaid
$行们 = @('```mermaid', 'flowchart LR')
foreach ($边 in ($边们.Values | Sort-Object { $_.来源 + $_.目标 })) {
    $行们 += "  $($边.来源)[$($边.来源)] --> $($边.目标)[$($边.目标)]"
}
$行们 += '```'
$文本 = $行们 -join "`n"

if ($输出) {
    Set-Content -Path $输出 -Value $文本 -Encoding UTF8
    Write-Host "依赖图已写入：$输出"
} else {
    Write-Host $文本
}
Write-Host "共 $($边们.Count) 条府间依赖边"
exit 0
