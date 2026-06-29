@echo off
setlocal

set "ROOT=%~dp0"
set "APP_EXE=%ROOT%src-tauri\target\release\Zhiji.exe"

if exist "%APP_EXE%" (
  start "" "%APP_EXE%" %*
  exit /b 0
)

echo Could not find the app executable:
echo   %APP_EXE%
echo.
echo Build it first, then run this script again:
echo   npm run tauri build
echo.
pause
exit /b 1
