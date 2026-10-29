@echo off
setlocal
REM GameCraft: double-click entry point. Delegates to start.ps1.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start.ps1" %*
if errorlevel 1 (
  echo.
  echo [gamecraft] start.ps1 exited with error - window held open, instacrash guard
  pause
  exit /b 1
)
endlocal
exit /b 0
