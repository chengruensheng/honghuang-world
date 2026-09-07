# 启动.ps1 —— 洪荒·世界 一键启动（PowerShell）
# 用法：
#   ./启动.ps1            启动后端(hm-bootstrap, 8321) 并自动打开浏览器访问 http://localhost:8321
#   ./启动.ps1 -Desktop   启动桌面壳(hm-desktop, 内嵌 WebView2 窗口)；后端/前端随壳启动
# 说明：
#   * 前端为 ES Module(type="module")，IDE 内置预览/webview 不执行 module 会白屏，
#     因此默认用"真浏览器"访问 localhost:8321（最稳、人人可用）；桌面壳用 WebView2 不受此限。
#   * 首次运行会自动 cargo build，稍慢；后续增量极快。
#   * LLM 请在界面右下角「接入供应商」配置(DeepSeek/GLM/Qwen/Kimi 等)，或写入 default.toml。

param(
    [switch]$Desktop
)
$ErrorActionPreference = 'Stop'
$根 = Split-Path -Parent $PSScriptRoot
Set-Location $根
Write-Host '== 洪荒·世界 一键启动 ==' -ForegroundColor Cyan

# 1) 编译
Write-Host '[1/3] 编译目标（首次较慢，请稍候）...'
if ($Desktop) {
    cargo build -p hm-desktop
    if ($LASTEXITCODE -ne 0) { Write-Host '编译失败(cargo hm-desktop)。' -ForegroundColor Red; exit 1 }
} else {
    cargo build -p hm-bootstrap
    if ($LASTEXITCODE -ne 0) { Write-Host '编译失败(cargo hm-bootstrap)。' -ForegroundColor Red; exit 1 }
}

# 2) 端口检测/启动
$端口 = 8321
$占用 = netstat -ano | findstr ":${端口}"
if ($占用 -and -not $Desktop) {
    Write-Host "[2/3] 端口 ${端口} 已被占用，跳过启动（后端或桌面壳可能已在运行）。" -ForegroundColor Yellow
} else {
    Write-Host '[2/3] 启动服务...'
    if ($Desktop) {
        Start-Process -FilePath (Join-Path $根 'target\debug\hm-desktop.exe')
    } else {
        Start-Process -FilePath (Join-Path $根 'target\debug\hm-bootstrap.exe')
    }
    # 等待端口就绪
    for ($i = 0; $i -lt 50; $i++) {
        if (netstat -ano | findstr ":${端口}") { break }
        Start-Sleep -Milliseconds 200
    }
}

# 3) 打开浏览器
Write-Host '[3/3] 打开访问入口...'
if (-not $Desktop) {
    $目标 = "http://localhost:${端口}"
    Write-Host "   浏览器访问: $目标" -ForegroundColor Green
    Start-Process $目标
} else {
    Write-Host '   桌面壳已启动（含前端+后端）。' -ForegroundColor Green
}

Write-Host '== 完成。若界面未配置 LLM，请在右下角「接入供应商」完成接入。 ==' -ForegroundColor Cyan
