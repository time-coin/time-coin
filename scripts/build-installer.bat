@echo off
setlocal EnableDelayedExpansion

:: ============================================================================
::  TIME Coin Wallet - Build Installer
::
::  Builds the wallet and packages it into a Windows installer using Inno Setup.
::
::  Prerequisites:
::    - Rust toolchain (cargo)
::    - Inno Setup 6 (https://jrsoftware.org/isdl.php)
::
::  Usage:  scripts\build-installer.bat
:: ============================================================================

echo.
echo ============================================
echo   TIME Coin Wallet - Installer Builder
echo ============================================
echo.

set "REPO_ROOT=%~dp0.."
set "ISCC="

:: ── Find Inno Setup compiler ────────────────────────────────────────────────
echo -- Locating Inno Setup compiler...

where iscc >nul 2>&1
if %errorlevel% equ 0 (
    set "ISCC=iscc"
) else (
    if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" (
        set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
    )
    if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" (
        set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
    )
)

if "!ISCC!"=="" (
    echo [ERROR] Inno Setup 6 not found.
    echo         Install from https://jrsoftware.org/isdl.php
    echo.
    pause
    exit /b 1
)
echo [OK] Found Inno Setup.

:: ── Build the wallet ────────────────────────────────────────────────────────
echo.
echo -- Building wallet in release mode...
echo    This may take several minutes.
echo.

pushd "%REPO_ROOT%"
cargo build --release
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Build failed. See output above.
    popd
    pause
    exit /b 1
)
popd
echo.
echo [OK] Build succeeded.

:: ── Convert PNG icon to ICO (if needed) ─────────────────────────────────────
echo.
echo -- Preparing icon...

set "ICO_FILE=%REPO_ROOT%\wallet-gui\assets\logo.ico"
set "PNG_FILE=%REPO_ROOT%\wallet-gui\assets\logo.png"

if not exist "!ICO_FILE!" (
    if exist "!PNG_FILE!" (
        echo    Converting logo.png to logo.ico...
        powershell -Command ^
            "Add-Type -AssemblyName System.Drawing; " ^
            "$png = [System.Drawing.Image]::FromFile('%PNG_FILE%'); " ^
            "$icon = [System.Drawing.Icon]::FromHandle(([System.Drawing.Bitmap]$png).GetHicon()); " ^
            "$fs = [System.IO.File]::Create('%ICO_FILE%'); " ^
            "$icon.Save($fs); " ^
            "$fs.Close(); " ^
            "$icon.Dispose(); " ^
            "$png.Dispose()"
        if exist "!ICO_FILE!" (
            echo [OK] Created logo.ico
        ) else (
            echo [WARN] Could not convert icon. Installer will use default icon.
        )
    )
)
if exist "!ICO_FILE!" (
    echo [OK] Icon ready.
)

:: ── Create output directory ─────────────────────────────────────────────────
if not exist "%REPO_ROOT%\installer" mkdir "%REPO_ROOT%\installer"

:: ── Compile installer ───────────────────────────────────────────────────────
echo.
echo -- Compiling installer...
echo.

"!ISCC!" "%REPO_ROOT%\scripts\installer.iss"
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Installer compilation failed.
    pause
    exit /b 1
)

echo.
echo ============================================
echo   Installer built successfully!
echo ============================================
echo.
echo   Output: installer\TIMECoinWallet-Setup-0.6.0.exe
echo.
pause
exit /b 0
