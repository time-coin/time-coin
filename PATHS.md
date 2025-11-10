# TIME Coin Directory Structure

## Standard Paths

### Windows Development (Git Bash)
```
~/projects/time-coin/
C:\Users\<username>\projects\time-coin\
```

### Ubuntu Server (Production)
```
~/time-coin/
/root/time-coin/
```

### Configuration Files
```
Windows:  ~/time-coin-node/config/testnet.toml
Ubuntu:   ~/time-coin-node/config/testnet.toml
          /root/time-coin-node/config/testnet.toml
```

### Binary Locations
```
Windows:  ~/projects/time-coin/target/release/timed.exe
Ubuntu:   /usr/local/bin/timed
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
