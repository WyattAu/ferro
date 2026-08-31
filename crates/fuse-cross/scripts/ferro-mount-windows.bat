@echo off
REM Ferro FUSE mount script for Windows (WinFSP)
REM Usage: ferro-mount-windows.bat [server-url] [mount-point] [token]
REM
REM Prerequisites:
REM   - ferro-fuse-cross.exe installed (cargo install ferro-fuse-cross)
REM   - WinFSP installed (https://winfsp.dev/rel/)
REM   - Set FERRO_TOKEN environment variable or pass as argument

setlocal

set SERVER_URL=%1
if "%SERVER_URL%"=="" set SERVER_URL=%FERRO_URL%
if "%SERVER_URL%"=="" set SERVER_URL=https://ferro.wyattau.com

set MOUNT_POINT=%2
if "%MOUNT_POINT%"=="" set MOUNT_POINT=%FERRO_MOUNT%
if "%MOUNT_POINT%"=="" set MOUNT_POINT=X:

set TOKEN=%3
if "%TOKEN%"=="" set TOKEN=%FERRO_TOKEN%

REM Check dependencies
where ferro-fuse-cross.exe >nul 2>&1
if errorlevel 1 (
    echo Error: ferro-fuse-cross.exe not found. Install with: cargo install ferro-fuse-cross
    exit /b 1
)

if "%TOKEN%"=="" (
    echo Error: No token provided. Set FERRO_TOKEN or pass as third argument.
    exit /b 1
)

REM Create mount point (virtual drive letter)
if not exist "%MOUNT_POINT%" mkdir "%MOUNT_POINT%"

echo Mounting Ferro at %MOUNT_POINT% from %SERVER_URL%
ferro-fuse-cross.exe --server-url "%SERVER_URL%" --mount "%MOUNT_POINT%" --token "%TOKEN%"

echo Ferro unmounted.
