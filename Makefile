.PHONY: build test install uninstall clean help

# Build the release binary
build:
	cargo build --release

# Run tests
test:
	cargo test

# Install the binary and systemd files
install: build
	@echo "Installing borg-timemachine..."
	sudo cp target/release/borg-timemachine /usr/local/bin/
	sudo mkdir -p /etc/borg
	@if [ ! -f /etc/borg/borg-config.yaml ]; then \
		echo "Generating default config at /etc/borg/borg-config.yaml"; \
		sudo /usr/local/bin/borg-timemachine generate-config /etc/borg/borg-config.yaml; \
	else \
		echo "Config already exists at /etc/borg/borg-config.yaml"; \
	fi
	@echo "Installing systemd files..."
	sudo cp systemd/borg-timemachine.service /etc/systemd/system/
	sudo cp systemd/borg-timemachine.timer /etc/systemd/system/
	sudo systemctl daemon-reload
	@echo ""
	@echo "Installation complete!"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Edit the config: sudo nano /etc/borg/borg-config.yaml"
	@echo "  2. Create passphrase: echo 'your-passphrase' | sudo tee /root/.borg-passphrase"
	@echo "  3. Set permissions: sudo chmod 600 /root/.borg-passphrase"
	@echo "  4. Initialize repo: sudo borg-timemachine init"
	@echo "  5. Backup the key: sudo borg key export <repo-path> ~/borg-key-backup.txt"
	@echo "  6. Enable timer: sudo systemctl enable --now borg-timemachine.timer"

# Initialize repository (assumes config exists)
init:
	@sudo test -f /root/.borg-passphrase || { \
		echo "Error: /root/.borg-passphrase not found"; \
		echo "Create it with: echo 'your-passphrase' | sudo tee /root/.borg-passphrase"; \
		echo "                sudo chmod 600 /root/.borg-passphrase"; \
		exit 1; \
	}
	sudo /usr/local/bin/borg-timemachine --config /etc/borg/borg-config.yaml init

# Enable and start the systemd timer
enable:
	sudo systemctl enable --now borg-timemachine.timer
	sudo systemctl status borg-timemachine.timer

# Disable the systemd timer
disable:
	sudo systemctl disable --now borg-timemachine.timer

# Run a manual backup
backup:
	sudo systemctl start borg-timemachine.service

# View backup logs
logs:
	sudo journalctl -u borg-timemachine.service -f

# Show timer status
status:
	sudo systemctl status borg-timemachine.timer
	@echo ""
	sudo systemctl list-timers | grep borg

# List all archives
list:
	sudo borg-timemachine --config /etc/borg/borg-config.yaml list

# Show repository info
info:
	sudo borg-timemachine --config /etc/borg/borg-config.yaml info

# Uninstall everything
uninstall:
	@echo "Stopping and disabling timer..."
	-sudo systemctl stop borg-timemachine.timer
	-sudo systemctl disable borg-timemachine.timer
	@echo "Removing systemd files..."
	-sudo rm /etc/systemd/system/borg-timemachine.service
	-sudo rm /etc/systemd/system/borg-timemachine.timer
	sudo systemctl daemon-reload
	@echo "Removing binary..."
	-sudo rm /usr/local/bin/borg-timemachine
	@echo "Uninstall complete!"
	@echo ""
	@echo "NOTE: Config file at /etc/borg/borg-config.yaml NOT removed"
	@echo "NOTE: Passphrase at /root/.borg-passphrase NOT removed"
	@echo "NOTE: Backup repository NOT removed"

# Clean build artifacts
clean:
	cargo clean

# Generate a new example config
generate-config:
	borg-timemachine generate-config borg-config-example.yaml
	@echo "Example config written to: borg-config-example.yaml"

# Show help
help:
	@echo "Borg Time Machine - Makefile targets"
	@echo ""
	@echo "Build & Test:"
	@echo "  make build              - Build release binary"
	@echo "  make test               - Run tests"
	@echo "  make clean              - Clean build artifacts"
	@echo ""
	@echo "Installation:"
	@echo "  make install            - Install binary and systemd files"
	@echo "  make init               - Initialize Borg repository"
	@echo "  make uninstall          - Remove binary and systemd files"
	@echo ""
	@echo "Operation:"
	@echo "  make enable             - Enable and start systemd timer"
	@echo "  make disable            - Disable systemd timer"
	@echo "  make backup             - Run manual backup"
	@echo "  make list               - List all archives"
	@echo "  make info               - Show repository info"
	@echo ""
	@echo "Monitoring:"
	@echo "  make status             - Show timer status"
	@echo "  make logs               - View backup logs (live)"
	@echo ""
	@echo "Utilities:"
	@echo "  make generate-config    - Generate example config file"
	@echo "  make help               - Show this help message"
