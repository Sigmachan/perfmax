#!/usr/bin/env bash
set -euo pipefail

BINARY=target/release/perfmax
BIN_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"
DESKTOP_DIR="$HOME/.local/share/applications"
SUDOERS=/etc/sudoers.d/perfmax

cargo build --release

install -Dm755 "$BINARY" "$BIN_DIR/perfmax"
install -Dm644 perfmax.desktop "$DESKTOP_DIR/perfmax.desktop"
mkdir -p "$SERVICE_DIR"
install -Dm644 perfmax.service "$SERVICE_DIR/perfmax.service"

# sudoers — allow perfmax to run tuning commands without password
sudo tee "$SUDOERS" > /dev/null <<'EOF'
sigmachan ALL=(ALL) NOPASSWD: /usr/bin/ryzenadj, /usr/bin/nvidia-smi, /usr/bin/cpupower, /usr/bin/sysctl, /usr/bin/taskset, /usr/bin/renice, /usr/bin/ionice, /usr/bin/tee, /usr/bin/sh
EOF
sudo chmod 440 "$SUDOERS"

systemctl --user daemon-reload
systemctl --user enable --now perfmax.service

echo "installed → $BIN_DIR/perfmax"
echo "service   → systemctl --user status perfmax"
