# TIME Coin Directory Structure

## Standard Paths (Bitcoin-style)

TIME Coin follows the Bitcoin convention for data directories:

### Linux/Mac
```
Data Directory:  ~/.timecoin/
Configuration:   ~/.timecoin/config/testnet.toml
Blockchain:      ~/.timecoin/data/blockchain/
Wallets:         ~/.timecoin/data/wallets/
Logs:            ~/.timecoin/logs/
```

### Windows
```
Data Directory:  %APPDATA%\timecoin\
                 (e.g., C:\Users\username\AppData\Roaming\timecoin)
Configuration:   %APPDATA%\timecoin\config\testnet.toml
Blockchain:      %APPDATA%\timecoin\data\blockchain\
Wallets:         %APPDATA%\timecoin\data\wallets\
Logs:            %APPDATA%\timecoin\logs\
```

## Environment Variable Override

You can override the default data directory:

```bash
# Linux/Mac
export TIME_COIN_DATA_DIR=/custom/path/.timecoin

# Windows (PowerShell)
$env:TIME_COIN_DATA_DIR = "C:\custom\path\timecoin"
```

## Development Paths

### Repository Location

#### Windows Development (Git Bash)
```
~/projects/time-coin/
C:\Users\<username>\projects\time-coin\
```

#### Ubuntu Server (Production)
```
~/time-coin/
/root/time-coin/
```

### Binary Locations

#### Windows Development
```
~/projects/time-coin/target/release/timed.exe
~/projects/time-coin/target/release/time-cli.exe
```

#### Linux Production
```
/usr/local/bin/timed
/usr/local/bin/time-cli
```

## Clone Commands

### Windows (Development)
```bash
cd ~/projects
git clone https://github.com/time-coin/time-coin.git
cd time-coin
```

### Ubuntu Server (Production)
```bash
cd ~
git clone https://github.com/time-coin/time-coin.git
cd time-coin
```

## Update Commands

### Windows
```bash
cd ~/projects/time-coin
git pull origin main
cargo build --release --bin timed
```

### Ubuntu Server
```bash
cd ~/time-coin
git pull origin main
cargo build --release --bin timed
sudo systemctl stop timed
sudo cp target/release/timed /usr/local/bin/
sudo systemctl start timed
```

## Migration from Old Paths

If you have data in the old location, you can migrate it:

### Linux
```bash
# Old path: /var/lib/time-coin or ~/time-coin-node
# New path: ~/.timecoin

# Move data
mv /var/lib/time-coin ~/.timecoin
# or
mv ~/time-coin-node ~/.timecoin
```

### Windows
```powershell
# Old path: %LOCALAPPDATA%\time-coin
# New path: %APPDATA%\timecoin

Move-Item "$env:LOCALAPPDATA\time-coin" "$env:APPDATA\timecoin"
```
