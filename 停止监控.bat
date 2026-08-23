@echo off
REM 洪荒 · 停止监控界面
REM 关闭所有 监控.exe 进程。如果进程不在跑会无副作用。

setlocal
taskkill /F /IM 监控.exe 2>nul
if %errorlevel% == 0 (
    echo 监控已停止。
) else (
    echo 监控未在跑（无需停止）。
)
pause