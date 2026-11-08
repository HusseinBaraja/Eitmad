@echo off
setlocal
set "build_pwsh=%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe"
if not exist "%build_pwsh%" set "build_pwsh=pwsh.exe"
"%build_pwsh%" -NoProfile -File "%~dp0scripts\release\build_windows.ps1" %*
set "build_result=%ERRORLEVEL%"
if "%~1"=="" pause
exit /b %build_result%
