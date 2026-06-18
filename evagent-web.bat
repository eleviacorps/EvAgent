@echo off
REM EvAgent Web — starts core engine and opens web GUI
setlocal enabledelayedexpansion
set COREDIR=%~dp0evagent-core
set PORT=9753

REM Kill stale core
taskkill /F /IM evagent-core.exe >nul 2>&1

echo [EvAgent] Starting core engine on port %PORT%...
start /B "" cmd /c "cd /d %COREDIR% && cargo run -- start --port %PORT%" > "%TEMP%\evagent-core.log" 2>&1

REM Wait for core
echo [EvAgent] Waiting for core...
:waitloop
timeout /t 2 /nobreak >nul
powershell -Command "try { $wc = New-Object System.Net.Sockets.TcpClient; $wc.Connect('127.0.0.1', %PORT%); $wc.Close(); exit 0 } catch { exit 1 }" >nul 2>&1
if errorlevel 1 goto waitloop

echo [EvAgent] Core ready. Opening web GUI...
start "" "%~dp0evagent-web\index.html"

echo [EvAgent] Web GUI opened in your browser.
echo [EvAgent] Close this window to shut down when done.
pause

REM Cleanup
taskkill /F /IM evagent-core.exe >nul 2>&1
