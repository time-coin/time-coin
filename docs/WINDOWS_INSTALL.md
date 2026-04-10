# Installing the TIME Coin Wallet on Windows

This guide walks through every step to install and run the TIME Coin Wallet on a Windows 10/11 computer.

---

## Quick Install (Automated)

If you already have `git` installed, you can use the automated dependency installer:

```powershell
git clone https://github.com/time-coin/time-coin.git
cd time-coin
scripts\install-deps.bat
```

Right-click `install-deps.bat` and select **Run as administrator**. It will install Visual Studio Build Tools and Rust for you. Once finished, continue to [Build and Run](#build-and-run).

---

## Manual Install (Step by Step)

### 1. Install Git

Download and install Git for Windows from [git-scm.com](https://git-scm.com/download/win).

Accept the default options during installation. When complete, open a new terminal and confirm:

```powershell
git --version
```

### 2. Install Visual Studio C++ Build Tools

Rust on Windows requires the MSVC C++ toolchain for linking. Install it using **one** of these methods:

#### Option A — winget (recommended)

Open PowerShell **as Administrator** and run:

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools `
    --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

#### Option B — manual download

1. Go to [https://visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Download and run **Build Tools for Visual Studio 2022**
3. In the installer, select **Desktop development with C++**
4. Click **Install**

This download is roughly 2–6 GB depending on components selected.

### 3. Install Rust

1. Go to [https://rustup.rs](https://rustup.rs)
2. Download and run `rustup-init.exe`
3. When prompted, press **1** to proceed with the default installation
4. After installation, **close and reopen your terminal**

Verify the installation:

```powershell
rustc --version
cargo --version
```

Both commands should print a version number. Rust 1.75 or higher is required.

### 4. Install Required Toolchain Components

The project uses `clippy` and `rustfmt`. Install them:

```powershell
rustup component add clippy rustfmt
```

---

## Build and Run

### Clone the Repository

```powershell
git clone https://github.com/time-coin/time-coin.git
cd time-coin
```

### Build the Wallet

```powershell
cargo build --release
```

The first build downloads dependencies and compiles everything — this can take **5–15 minutes** depending on your hardware.

### Run the Wallet

```powershell
.\target\release\wallet-gui.exe
```

Or build and run in one step:

```powershell
cargo run --release
```

---

## Verify Your Setup

Run the full test suite to make sure everything is working:

```powershell
cargo test --workspace
```

Run the linter:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Where Wallet Data is Stored

On first launch, the wallet creates its data directory at:

```
%USERPROFILE%\.time-wallet\
```

This directory contains your configuration, encrypted wallet files, and local database. **Back up this directory** to preserve your wallet.

---

## Troubleshooting

### `cargo` or `rustc` is not recognized

Close your terminal and open a new one. The Rust installer adds `%USERPROFILE%\.cargo\bin` to your PATH, but this only takes effect in new terminal sessions.

### Linker errors (`link.exe not found`)

Visual Studio Build Tools are not installed or not detected. Reinstall using [step 2](#2-install-visual-studio-c-build-tools), then restart your terminal.

### Build fails with network errors

Cargo downloads crates from [crates.io](https://crates.io). Make sure you have an internet connection and that your firewall/proxy allows HTTPS traffic to `crates.io` and `github.com`.

### Build is very slow

First builds compile all dependencies and are expected to take 5–15 minutes. Subsequent builds reuse cached artifacts and are much faster. Using `cargo check --workspace` instead of a full build is a fast way to validate code changes.

### The window doesn't appear / crashes on launch

Make sure your graphics drivers are up to date. The wallet uses [egui](https://github.com/emilk/egui), which requires a working GPU or software renderer. On systems without a GPU, try setting:

```powershell
$env:WGPU_BACKEND = "gl"
.\target\release\wallet-gui.exe
```

---

## Updating

To update to the latest version:

```powershell
cd time-coin
git pull
cargo build --release
```

---

## Uninstalling

1. Delete the cloned repository folder
2. Remove wallet data: `Remove-Item -Recurse ~\.time-wallet` (⚠️ this deletes your wallet — back up first)
3. Optionally uninstall Rust: `rustup self uninstall`
