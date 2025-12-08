# Borg Time Machine

A Time Machine-style automated backup system using BorgBackup, written in Rust.

## Features

- Automated hourly backups with systemd timer
- YAML configuration for backup jobs
- Time Machine-style retention (hourly, daily, weekly, monthly, yearly)
- Deduplication and compression via BorgBackup
- Email notifications on failures

## Prerequisites

- BorgBackup (`sudo dnf install borgbackup` or equivalent)
- Rust toolchain (for building)

## Quick Start

```bash
# Build
cargo build --release

# Install binary and systemd files
make install

# Edit config
sudo vim /etc/borg/borg-config.yaml

# Create passphrase
echo 'your-strong-passphrase' | sudo tee /root/.borg-passphrase
sudo chmod 600 /root/.borg-passphrase

# Initialize repository
make init

# IMPORTANT: Backup your key!
sudo borg key export /path/to/repo ~/borg-key-backup.txt

# Enable automatic backups
make enable
```

## Usage

```bash
make backup    # Run backup now
make list      # List archives
make info      # Repository info
make status    # Timer status
make logs      # View logs (live)
```

## Configuration

Edit `/etc/borg/borg-config.yaml`:

```yaml
repository:
  path: /mnt/backup/borg    # Local or user@host:/path for remote
  encryption: repokey-blake2

jobs:
  - name: home
    source: /home
    destination: home
    enabled: true
    exclude:
      - '/home/*/.cache'

retention:
  within: 24h
  hourly: 24
  daily: 7
  weekly: 4
  monthly: 6
  yearly: 2
```

## Restore Files

```bash
# Mount repository
sudo mkdir -p /mnt/borg
sudo borg-timemachine mount /mnt/borg

# Browse and copy files
ls /mnt/borg
cp /mnt/borg/<archive>/path/to/file /tmp/restored

# Unmount
sudo fusermount -u /mnt/borg
```

## Makefile Targets

```
make build      Build release binary
make install    Install binary + systemd files
make init       Initialize borg repository
make enable     Enable systemd timer
make disable    Disable systemd timer
make backup     Run manual backup
make list       List archives
make info       Repository info
make status     Timer status
make logs       View logs
make uninstall  Remove installation
make help       Show all targets
```
