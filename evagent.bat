@echo off
REM EvAgent — One-command launcher for core + TUI
REM Usage: evagent [--port PORT]

setlocal enabledelayedexpansion
set COREDIR=%~dp0evagent-core
set TUIDIR=%~dp0evagent-tui
set PORT=9753

REM Parse optional --port argument
if not "%1"=="" (
    if "%1"=="--port" (
        if not "%2"=="" set PORT=%2
    )
)

REM Kill any stale core process
taskkill /F /IM evagent-core.exe >nul 2>&1

echo [EvAgent] Starting core engine on port %PORT%...
start "EvAgent Core" cmd /c "cd /d %COREDIR% && cargo run -- start --port %PORT%"

REM Wait for core to be ready
echo [EvAgent] Waiting for core to start...
:waitloop
timeout /t 2 /nobreak >nul
powershell -Command "try { $wc = New-Object System.Net.Sockets.TcpClient; $wc.Connect('127.0.0.1', %PORT%); $wc.Close(); exit 0 } catch { exit 1 }" >nul 2>&1
if errorlevel 1 goto waitloop

echo [EvAgent] Core ready. Launching TUI...
cd /d %TUIDIR%
cargo run -- --port %PORT%

REM When TUI exits, kill the core
echo [EvAgent] Shutting down core...
taskkill /F /IM evagent-core.exe >nul 2>&1
