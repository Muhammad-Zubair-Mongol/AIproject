@echo off
set PATH=%PATH%;%USERPROFILE%\.cargo\bin
echo ================================
echo  MEETING MIND - QUICK DEV START
echo ================================
echo.

REM Kill any existing processes that might hold cargo locks
taskkill /f /im god-v8.exe 2>nul
taskkill /f /im tauri.exe 2>nul

cd /d "%~dp0"

echo Step 1: Running Tauri development environment...
echo This will start both the frontend and the Rust backend.
npx tauri dev

echo.
echo Done!
pause
