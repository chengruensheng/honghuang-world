# 启动.ps1 —— 后端一键启动入口（转发到 .传承/门禁/启动.ps1）
# 用法：
#   ./启动.ps1            编译并启动后端(hm-bootstrap)，数据服务监听 127.0.0.1:8321（纯 API）
# 说明：
#   * 本文件仅作「运行入口」保留（保持 ./启动.ps1 旧习惯不变），实际逻辑唯一源见
#     .传承/门禁/启动.ps1，避免两处实现漂移。
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot '.传承\门禁\启动.ps1') @args
exit $LASTEXITCODE
