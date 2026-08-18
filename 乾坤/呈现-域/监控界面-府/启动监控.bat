@echo off
chcp 65001 > nul
setlocal
cd /d "%~dp0"
echo.
echo  ======== 洪荒 · 监控界面 ========
echo  端口:8080（也可 python server.py 9090 自定义）
echo  浏览器:http://127.0.0.1:8080
echo  停止:Ctrl+C
echo.
where python > nul 2> nul
if errorlevel 1 (
  where py > nul 2> nul
  if errorlevel 1 (
    echo  [ERROR] 需先装 Python 3.10+ (https://www.python.org/downloads/)
    pause
    exit /b 1
  )
  py -3 server.py %1
) else (
  python server.py %1
)
pause