@echo off
setlocal EnableDelayedExpansion

:: ============================================================================
::  TIME Coin Wallet - Windows Dependency Installer
::
::  Installs everything needed to build and run the wallet:
::    1. Visual Studio C++ Build Tools  (MSVC linker and Windows SDK)
::    2. Rust toolchain via rustup       (stable channel + clippy, rustfmt)
::
::  Usage:  Right-click > Run as Administrator, or from an admin terminal:
::          scripts\install-deps.bat
:: ============================================================================

echo.
echo ============================================
echo   TIME Coin Wallet - Dependency Installer
echo ============================================
echo.

:: ── Admin check ─────────────────────────────────────────────────────────────
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] This script must be run as Administrator.
    echo         Right-click the file and select "Run as administrator".
    echo.
    pause
    exit /b 1
)

:: ── Check for winget ────────────────────────────────────────────────────────
where winget >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] winget is not available on this system.
    echo         Install "App Installer" from the Microsoft Store, then re-run.
    echo         https://aka.ms/getwinget
    echo.
    pause
    exit /b 1
)

echo [OK] winget found.

:: ── Visual Studio Build Tools ───────────────────────────────────────────────
echo.
echo -- Checking for Visual Studio C++ Build Tools...

set "VS_INSTALLED=0"

:: Check for Build Tools or full Visual Studio with C++ workload
for /f "tokens=*" %%i in ('where cl 2^>nul') do set "VS_INSTALLED=1"
if "!VS_INSTALLED!"=="0" (
    if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" (
        for /f "tokens=*" %%i in ('"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2^>nul') do (
            if not "%%i"=="" set "VS_INSTALLED=1"
        )
    )
)

if "!VS_INSTALLED!"=="1" (
    echo [OK] Visual Studio C++ Build Tools already installed.
) else (
    echo [INSTALLING] Visual Studio Build Tools with C++ workload...
    echo              This may take 10-20 minutes on first install.
    echo.
    winget install Microsoft.VisualStudio.2022.BuildTools ^
        --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" ^
        --accept-source-agreements --accept-package-agreements
    if !errorlevel! neq 0 (
        echo.
        echo [ERROR] Failed to install Visual Studio Build Tools.
        echo         Try installing manually from https://visualstudio.microsoft.com/visual-cpp-build-tools/
        echo         Select "Desktop development with C++".
        echo.
        pause
        exit /b 1
    )
    echo [OK] Visual Studio Build Tools installed.
    echo      Refreshing environment PATH...
    :: Refresh PATH so rustup-init can detect MSVC
    for /f "tokens=2*" %%A in ('reg query "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v Path 2^>nul') do set "PATH=%%B;!PATH!"
    for /f "tokens=2*" %%A in ('reg query "HKCU\Environment" /v Path 2^>nul') do set "PATH=%%B;!PATH!"
)

:: ── Rust toolchain ──────────────────────────────────────────────────────────
echo.
echo -- Checking for Rust toolchain...

where rustup >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] rustup already installed.
    echo      Ensuring stable toolchain with required components...
    rustup toolchain install stable -c clippy -c rustfmt
    rustup default stable
    echo [OK] Rust toolchain is up to date.
) else (
    echo [INSTALLING] Rust via rustup...
    echo.

    :: Download rustup-init.exe
    set "RUSTUP_INIT=%TEMP%\rustup-init.exe"
    echo      Downloading rustup-init.exe...
    where curl >nul 2>&1
    if !errorlevel! equ 0 (
        curl -sSfL -o "!RUSTUP_INIT!" https://win.rustup.rs/x86_64
    ) else (
        powershell -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile '!RUSTUP_INIT!'"
    )
    if not exist "!RUSTUP_INIT!" (
        echo [ERROR] Failed to download rustup-init.exe.
        echo         Download manually from https://rustup.rs
        echo.
        pause
        exit /b 1
    )

    :: Install Rust (default stable, adds to PATH)
    "!RUSTUP_INIT!" -y --default-toolchain stable -c clippy -c rustfmt
    if !errorlevel! neq 0 (
        echo [ERROR] Rust installation failed. See output above for details.
        echo         Try installing manually from https://rustup.rs
        echo.
        pause
        exit /b 1
    )

    del "!RUSTUP_INIT!" 2>nul

    :: Add cargo to current session PATH
    set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
    echo [OK] Rust installed successfully.
)

:: ── Verify installation ────────────────────────────────────────────────────
echo.
echo -- Verifying installations...
echo.

set "ALL_OK=1"

where rustc >nul 2>&1
if %errorlevel% equ 0 (
    for /f "tokens=*" %%v in ('rustc --version 2^>nul') do echo [OK] %%v
) else (
    echo [WARN] rustc not found in current session.
    echo        Close and reopen your terminal for PATH changes to take effect.
    set "ALL_OK=0"
)

where cargo >nul 2>&1
if %errorlevel% equ 0 (
    for /f "tokens=*" %%v in ('cargo --version 2^>nul') do echo [OK] %%v
) else (
    echo [WARN] cargo not found in current session.
    set "ALL_OK=0"
)

:: ── Summary ─────────────────────────────────────────────────────────────────
echo.
echo ============================================
if "!ALL_OK!"=="1" (
    echo   All dependencies installed!
    echo ============================================
    echo.
    echo   Next steps:
    echo     1. cd into the project root
    echo     2. cargo build --release
    echo     3. .\target\release\wallet-gui.exe
) else (
    echo   Installation complete!
    echo ============================================
    echo.
    echo   ** Restart your terminal ** so PATH changes
    echo   take effect, then run:
    echo     1. cd into the project root
    echo     2. cargo build --release
    echo     3. .\target\release\wallet-gui.exe
)
echo.
pause
exit /b 0
