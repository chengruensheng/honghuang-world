# 启动.ps1 —— 洪荒·世界 后端一键启动（PowerShell）
# 用法：
#   ./启动.ps1            编译并启动后端(hm-bootstrap)，数据服务监听 127.0.0.1:8321（纯 API）
# 说明：
#   * 本项目为纯后端：不含任何前端代码；对外呈现面（CORS / 静态托管 / SSE 上限）由
#     工作目录下的 对外契约.toml 声明，增删客户端不改后端代码。
#   * 首次运行会自动 cargo build，稍慢；后续增量极快。
#   * LLM 请在 default.toml 的 [llm] 段配置（密钥推荐写 env:变量名，不入库）。
$ErrorActionPreference = 'Stop'
$根 = $PSScriptRoot
Set-Location $根
Write-Host '== 洪荒·世界 后端一键启动 ==' -ForegroundColor Cyan

# 1) 编译
Write-Host '[1/2] 编译后端（首次较慢，请稍候）...'
cargo build -p hm-bootstrap
if ($LASTEXITCODE -ne 0) { Write-Host '编译失败(cargo hm-bootstrap)。' -ForegroundColor Red; exit 1 }

# 2) 端口检测/启动
$端口 = 8321
$占用 = netstat -ano | findstr ":${端口}"
if ($占用) {
    Write-Host "[2/2] 端口 ${端口} 已被占用，跳过启动（后端可能已在运行）。" -ForegroundColor Yellow
} else {
    Write-Host '[2/2] 启动后端服务...'
    Start-Process -FilePath (Join-Path $根 'target\debug\hm-bootstrap.exe')
    for ($i = 0; $i -lt 50; $i++) {
        if (netstat -ano | findstr ":${端口}") { break }
        Start-Sleep -Milliseconds 200
    }
    Write-Host "   数据服务已启动: http://127.0.0.1:${端口}（纯 API）" -ForegroundColor Green
}

Write-Host '== 完成。请在 default.toml 的 [llm] 段配置大模型后使用。 ==' -ForegroundColor Cyan
