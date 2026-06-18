@echo off
REM EvAgent one-command launcher for Windows
REM Starts the Rust core on port 9753 and opens the Web GUI in the default browser.

setlocal
set EVAGENT_ROOT=%~dp0

echo [evagent] Starting core engine...
start "evagent-core" cmd /k "cd /d %EVAGENT_ROOT%evagent-core && cargo run -- release start"

REM Wait for the WS server to come up
timeout /t 3 /nobreak >nul

echo [evagent] Opening Web GUI...
start "" "%EVAGENT_ROOT%evagent-web\index.html"

echo [evagent] Launching TUI in this window...
cd /d %EVAGENT_ROOT%evagent-opentui
bun src\index.tsx

endlocal
