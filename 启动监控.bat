@echo off
REM 洪荒 · 启动监控界面（端口8080）
REM 双击运行。窗口保持打开，进程与终端解耦（不会被工具环境回收）。
REM 关闭本窗口或按 Ctrl+C = 停止监控。
REM 重启前若残留进程，先用 停止监控.bat 清理。

setlocal
set "PROJECT_ROOT=%~dp0"
set "EXE=%PROJECT_ROOT%道果树\构建物-域\debug\监控.exe"
set "LOG_OUT=%PROJECT_ROOT%临时文件夹\监控.out.log"
set "LOG_ERR=%PROJECT_ROOT%临时文件夹\监控.err.log"

REM 三源文件所在目录（.上下文/事件流.jsonl 等）
set "WORKSPACE_ROOT=%PROJECT_ROOT%"

if not exist "%EXE%" (
    echo 监控二进制不存在，开始编译 ...
    pushd "%PROJECT_ROOT%"
    cargo build -p jiankong_fu
    popd
)

if not exist "%EXE%" (
    echo 编译失败，请查看上方输出。
    pause
    exit /b 1
)

echo 启动监控界面于 http://localhost:8080/
echo 工作目录 = %WORKSPACE_ROOT%
echo 日志  stdout  -^> %LOG_OUT%
echo       stderr  -^> %LOG_ERR%
echo 关闭本窗口或按 Ctrl+C 停止。
echo.

cd /d "%WORKSPACE_ROOT%"
"%EXE%" 8080

echo.
echo 监控已退出。码=%errorlevel%
pause