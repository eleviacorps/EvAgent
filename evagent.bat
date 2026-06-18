@echo off
REM EvAgent Launcher — starts core + TUI
setlocal enabledelayedexpansion
set COREDIR=%~dp0evagent-core
set TUIDIR=%~dp0evagent-opentui
set PORT=9753

if not "%1"=="" if "%1"=="--web" goto web
if not "%1"=="" if "%1"=="--port" if not "%2"=="" set PORT=%2

:web
REM Kill stale core
taskkill /F /IM evagent-core.exe >nul 2>&1

echo [EvAgent] Starting core on port %PORT%...
start /B "" cmd /c "cd /d %COREDIR% && cargo run -- start --port %PORT%" > "%TEMP%\evagent-core.log" 2>&1

REM Wait for core
echo [EvAgent] Waiting for core...
:waitloop
timeout /t 2 /nobreak >nul
powershell -Command "try { $wc = New-Object System.Net.Sockets.TcpClient; $wc.Connect('127.0.0.1', %PORT%); $wc.Close(); exit 0 } catch { exit 1 }" >nul 2>&1
if errorlevel 1 goto waitloop

echo [EvAgent] Core ready.
if "%1"=="--web" goto launch_web

:launch_tui
echo [EvAgent] Starting OpenTUI terminal app...
cd /d %TUIDIR%
bun run src/entry.ts
goto cleanup

:launch_web
echo [EvAgent] Opening web GUI...
start "" "%~dp0evagent-web\index.html"
echo [EvAgent] Press any key to shutdown.
pause >nul

:cleanup
taskkill /F /IM evagent-core.exe >nul 2>&1
