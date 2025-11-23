@echo off
REM Quick build script for TIME Coin main binaries (Windows)

setlocal

set MODE=%1
if "%MODE%"=="" set MODE=debug

if "%MODE%"=="release" (
    echo 🔨 Building TIME Coin (Release mode^)...
    cargo build --release --bin timed --bin time-cli
    if %ERRORLEVEL% EQU 0 (
        echo ✅ Binaries built:
        echo    📦 timed:    target\release\timed.exe
        echo    📦 time-cli: target\release\time-cli.exe
    )
) else (
    echo 🔨 Building TIME Coin (Debug mode^)...
    cargo build --bin timed --bin time-cli
    if %ERRORLEVEL% EQU 0 (
        echo ✅ Binaries built:
        echo    📦 timed:    target\debug\timed.exe
        echo    📦 time-cli: target\debug\time-cli.exe
    )
)

echo.
echo 💡 Usage:
echo    Debug:   build.bat
echo    Release: build.bat release

endlocal
