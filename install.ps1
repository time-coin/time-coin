<#
.SYNOPSIS
    Installs the TIME Coin Wallet on Windows.

.DESCRIPTION
    Builds the wallet from source (or uses a pre-built binary) and installs it
    to Program Files with Start Menu and Desktop shortcuts. Requires Rust 1.75+.

.PARAMETER SkipBuild
    Skip the build step and use an existing binary from target\release.

.PARAMETER InstallDir
    Override the default installation directory.

.PARAMETER NoDesktopShortcut
    Skip creating a Desktop shortcut.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -SkipBuild
    .\install.ps1 -InstallDir "D:\Apps\TIMEWallet"
#>
param(
    [switch]$SkipBuild,
    [string]$InstallDir,
    [switch]$NoDesktopShortcut
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Constants ────────────────────────────────────────────────────────────────
$AppName       = "TIME Coin Wallet"
$ExeName       = "wallet-gui.exe"
$Publisher      = "TIME Coin Contributors"
$Version        = "0.1.0"
$RepoRoot       = $PSScriptRoot
$ReleaseBinary  = Join-Path $RepoRoot "target\release\$ExeName"
$LogoSource     = Join-Path $RepoRoot "wallet-gui\assets\logo.png"
$UninstallRegKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\TIMECoinWallet"

if (-not $InstallDir) {
    $InstallDir = Join-Path $env:ProgramFiles "TIME Coin Wallet"
}

# ── Helper functions ─────────────────────────────────────────────────────────
function Write-Step($msg) { Write-Host "`n>> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "   $msg" -ForegroundColor Green }
function Write-Warn($msg)  { Write-Host "   $msg" -ForegroundColor Yellow }

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function New-Shortcut {
    param([string]$ShortcutPath, [string]$TargetPath, [string]$IconPath, [string]$Description)
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $TargetPath
    $shortcut.Description = $Description
    $shortcut.WorkingDirectory = Split-Path $TargetPath
    if ($IconPath -and (Test-Path $IconPath)) {
        $shortcut.IconLocation = "$IconPath, 0"
    }
    $shortcut.Save()
}

# ── Pre-flight checks ───────────────────────────────────────────────────────
Write-Host ""
Write-Host "============================================" -ForegroundColor Magenta
Write-Host "   $AppName Installer v$Version" -ForegroundColor Magenta
Write-Host "============================================" -ForegroundColor Magenta

if (-not (Test-Admin)) {
    Write-Host "`nERROR: This installer requires Administrator privileges." -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as Administrator', then try again.`n" -ForegroundColor Yellow
    exit 1
}

# ── Build ────────────────────────────────────────────────────────────────────
if (-not $SkipBuild) {
    Write-Step "Checking Rust toolchain..."
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    if (-not $rustc) {
        Write-Host "`nERROR: Rust is not installed." -ForegroundColor Red
        Write-Host "Install it from https://rustup.rs and re-run this script.`n" -ForegroundColor Yellow
        exit 1
    }
    $rustVersion = & rustc --version
    Write-Ok "Found $rustVersion"

    Write-Step "Building $AppName (release mode)..."
    Write-Host "   This may take several minutes on first build." -ForegroundColor Gray
    Push-Location $RepoRoot
    try {
        & cargo build --release 2>&1 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`nERROR: Build failed. See output above.`n" -ForegroundColor Red
            exit 1
        }
        Write-Ok "Build succeeded."
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path $ReleaseBinary)) {
    Write-Host "`nERROR: Binary not found at $ReleaseBinary" -ForegroundColor Red
    Write-Host "Run the script without -SkipBuild, or build manually first.`n" -ForegroundColor Yellow
    exit 1
}

# ── Install files ────────────────────────────────────────────────────────────
Write-Step "Installing to $InstallDir..."

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Copy binary
Copy-Item -Path $ReleaseBinary -Destination (Join-Path $InstallDir $ExeName) -Force
Write-Ok "Copied $ExeName"

# Copy logo
$AssetsDir = Join-Path $InstallDir "assets"
if (-not (Test-Path $AssetsDir)) {
    New-Item -ItemType Directory -Path $AssetsDir -Force | Out-Null
}
if (Test-Path $LogoSource) {
    Copy-Item -Path $LogoSource -Destination (Join-Path $AssetsDir "logo.png") -Force
    Write-Ok "Copied logo.png"
}

# Create uninstall script
$UninstallScript = Join-Path $InstallDir "uninstall.ps1"
@"
<#
.SYNOPSIS
    Uninstalls the TIME Coin Wallet from this computer.
#>
Set-StrictMode -Version Latest
`$ErrorActionPreference = "Stop"

`$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
`$principal = New-Object Security.Principal.WindowsPrincipal(`$identity)
if (-not `$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "ERROR: Uninstall requires Administrator privileges." -ForegroundColor Red
    exit 1
}

Write-Host "Uninstalling TIME Coin Wallet..." -ForegroundColor Cyan

# Remove Start Menu shortcut
`$startMenuDir = Join-Path ([Environment]::GetFolderPath("CommonStartMenu")) "Programs\TIME Coin Wallet"
if (Test-Path `$startMenuDir) {
    Remove-Item -Path `$startMenuDir -Recurse -Force
    Write-Host "  Removed Start Menu shortcut." -ForegroundColor Green
}

# Remove Desktop shortcut
`$desktopShortcut = Join-Path ([Environment]::GetFolderPath("CommonDesktopDirectory")) "TIME Coin Wallet.lnk"
if (Test-Path `$desktopShortcut) {
    Remove-Item -Path `$desktopShortcut -Force
    Write-Host "  Removed Desktop shortcut." -ForegroundColor Green
}

# Remove registry entry
`$regKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\TIMECoinWallet"
if (Test-Path `$regKey) {
    Remove-Item -Path `$regKey -Force
    Write-Host "  Removed registry entry." -ForegroundColor Green
}

# Remove install directory (self-destruct)
`$installDir = Split-Path -Parent `$MyInvocation.MyCommand.Path
Write-Host "  Removing `$installDir..." -ForegroundColor Green
Start-Process -FilePath "cmd.exe" -ArgumentList "/c timeout /t 2 /nobreak >nul & rmdir /s /q `"`$installDir`"" -WindowStyle Hidden
Write-Host "`nTIME Coin Wallet has been uninstalled.`n" -ForegroundColor Cyan
"@ | Set-Content -Path $UninstallScript -Encoding UTF8
Write-Ok "Created uninstall.ps1"

# ── Shortcuts ────────────────────────────────────────────────────────────────
Write-Step "Creating shortcuts..."

$TargetExe = Join-Path $InstallDir $ExeName
$IconFile = Join-Path $AssetsDir "logo.png"

# Start Menu
$StartMenuDir = Join-Path ([Environment]::GetFolderPath("CommonStartMenu")) "Programs\TIME Coin Wallet"
if (-not (Test-Path $StartMenuDir)) {
    New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
}
$StartMenuLink = Join-Path $StartMenuDir "TIME Coin Wallet.lnk"
New-Shortcut -ShortcutPath $StartMenuLink -TargetPath $TargetExe -IconPath $IconFile -Description $AppName
Write-Ok "Start Menu shortcut created."

# Desktop (optional)
if (-not $NoDesktopShortcut) {
    $DesktopLink = Join-Path ([Environment]::GetFolderPath("CommonDesktopDirectory")) "TIME Coin Wallet.lnk"
    New-Shortcut -ShortcutPath $DesktopLink -TargetPath $TargetExe -IconPath $IconFile -Description $AppName
    Write-Ok "Desktop shortcut created."
}

# ── Registry (Add/Remove Programs) ──────────────────────────────────────────
Write-Step "Registering in Add/Remove Programs..."

if (-not (Test-Path $UninstallRegKey)) {
    New-Item -Path $UninstallRegKey -Force | Out-Null
}

$regValues = @{
    DisplayName    = $AppName
    DisplayVersion = $Version
    Publisher      = $Publisher
    InstallLocation = $InstallDir
    UninstallString = "powershell.exe -ExecutionPolicy Bypass -File `"$UninstallScript`""
    NoModify       = 1
    NoRepair       = 1
}

foreach ($key in $regValues.Keys) {
    $val = $regValues[$key]
    if ($val -is [int]) {
        Set-ItemProperty -Path $UninstallRegKey -Name $key -Value $val -Type DWord
    } else {
        Set-ItemProperty -Path $UninstallRegKey -Name $key -Value $val -Type String
    }
}

# Set icon if available
if (Test-Path $IconFile) {
    Set-ItemProperty -Path $UninstallRegKey -Name "DisplayIcon" -Value $TargetExe -Type String
}

Write-Ok "Registered successfully."

# ── Done ─────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "   Installation complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
Write-Host "   Location:  $InstallDir" -ForegroundColor White
Write-Host "   Launch:    Start Menu > $AppName" -ForegroundColor White
Write-Host "   Uninstall: Add/Remove Programs or run:" -ForegroundColor White
Write-Host "              $UninstallScript" -ForegroundColor Gray
Write-Host ""
