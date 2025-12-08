#!/bin/bash
# Helper script to mount Borg repository for browsing
# Usage: ./borg-mount.sh [mount-point]

set -e

MOUNT_POINT="${1:-/mnt/borg-browse}"
CONFIG_FILE="/etc/borg/borg-config.yaml"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root"
    echo "Usage: sudo $0 [mount-point]"
    exit 1
fi

# Check if borg-timemachine is installed
if ! command -v borg-timemachine &> /dev/null; then
    echo "Error: borg-timemachine not found"
    echo "Install it with: make install"
    exit 1
fi

# Check if config exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file not found at $CONFIG_FILE"
    exit 1
fi

# Create mount point if it doesn't exist
if [ ! -d "$MOUNT_POINT" ]; then
    echo "Creating mount point: $MOUNT_POINT"
    mkdir -p "$MOUNT_POINT"
fi

# Check if already mounted
if mountpoint -q "$MOUNT_POINT"; then
    echo "Warning: $MOUNT_POINT is already mounted"
    echo "Unmount with: fusermount -u $MOUNT_POINT"
    exit 1
fi

echo "Mounting Borg repository to: $MOUNT_POINT"
borg-timemachine --config "$CONFIG_FILE" mount "$MOUNT_POINT"

echo ""
echo "✓ Repository mounted successfully!"
echo ""
echo "Browse backups:"
echo "  ls $MOUNT_POINT"
echo "  cd $MOUNT_POINT/<archive-name>/..."
echo ""
echo "Unmount when done:"
echo "  fusermount -u $MOUNT_POINT"
echo "  # or: sudo $0 --umount"
