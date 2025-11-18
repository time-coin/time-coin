@echo off
REM TIME Coin Wallet Launcher
REM This script launches the TIME Coin wallet GUI

echo Starting TIME Coin Wallet...
echo.

REM Change to the bin directory
cd /d "%~dp0bin"

REM Launch the wallet
start "" "time-coin-wallet.exe"

REM Check if launch was successful
if errorlevel 1 (
    echo.
    echo ERROR: Failed to launch TIME Coin Wallet!
    echo.
    echo Troubleshooting:
    echo   1. Make sure time-coin-wallet.exe exists in the bin folder
    echo   2. Check if Windows Defender is blocking the application
    echo   3. Right-click time-coin-wallet.exe ^> Properties ^> Unblock
    echo.
    pause
) else (
    echo Wallet launched successfully!
    timeout /t 2 >nul
)
