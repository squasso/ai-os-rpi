#!/usr/bin/env bash
# ============================================================
#  AI-OS — Build script principale
#  Compila i binari Rust e prepara l'overlay pronto per il deploy.
#
#  Output:
#    dist/overlay/      → file da copiare sull'immagine RPi
#    dist/ai-os.tar.gz  → archivio dell'overlay
#    dist/firstrun.sh   → script di primo avvio (per rpi-imager)
#
#  Prerequisiti: cargo, cross (cargo install cross)
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$ROOT_DIR/dist"
TARGET="aarch64-unknown-linux-gnu"

echo "╔══════════════════════════════════════╗"
echo "║   AI-OS — Build per Raspberry Pi 4   ║"
echo "╚══════════════════════════════════════╝"

# ── 1. Cross-compila i binari Rust ──────────────────────────────────────────
echo
echo "▶ 1/4  Compilazione Rust (target: $TARGET)"
cd "$ROOT_DIR"

if command -v cross &>/dev/null; then
    cross build --release --target "$TARGET"
else
    echo "    ⚠  'cross' non trovato — uso cargo (binari x86, non per RPi)"
    cargo build --release
    TARGET="$(rustc -vV | grep host | awk '{print $2}')"
fi

BINS="$ROOT_DIR/target/$TARGET/release"

# ── 2. Crea il filesystem overlay ───────────────────────────────────────────
echo
echo "▶ 2/4  Creazione overlay filesystem"
OVERLAY="$OUT_DIR/overlay"
rm -rf "$OVERLAY"
mkdir -p "$OVERLAY/usr/local/bin"
mkdir -p "$OVERLAY/usr/local/share/ai-os"
mkdir -p "$OVERLAY/etc/xdg/ai-os"
mkdir -p "$OVERLAY/etc/systemd/user"
mkdir -p "$OVERLAY/etc/wayfire"
mkdir -p "$OVERLAY/home/pi/.config/ai-os"

# Binari
cp "$BINS/ai-daemon" "$OVERLAY/usr/local/bin/"
cp "$BINS/ai-shell"  "$OVERLAY/usr/local/bin/"

# Script di aggiornamento
cp "$ROOT_DIR/scripts/ai-os-update.sh" "$OVERLAY/usr/local/bin/ai-os-update"
chmod +x "$OVERLAY/usr/local/bin/ai-os-update"

# Versione (letta da Cargo.toml)
VERSION=$(grep '^version' "$ROOT_DIR/Cargo.toml" | grep -v 'workspace' | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "0.1.0")
# Oppure dalla sezione [workspace.package]
if [[ -z "$VERSION" || "$VERSION" == "0.1.0" ]]; then
    VERSION=$(grep -A5 '\[workspace.package\]' "$ROOT_DIR/Cargo.toml" | grep 'version' | sed 's/.*"\(.*\)".*/\1/' | head -1 || echo "0.1.0")
fi
echo "$VERSION" > "$OVERLAY/usr/local/share/ai-os/VERSION"
echo "    Versione: $VERSION"

# Systemd services e timer
cp "$ROOT_DIR/systemd/ai-daemon.service"      "$OVERLAY/etc/systemd/user/"
cp "$ROOT_DIR/systemd/ai-os-update.service"   "$OVERLAY/etc/systemd/user/"
cp "$ROOT_DIR/systemd/ai-os-update.timer"     "$OVERLAY/etc/systemd/user/"

# Configurazione default
cp "$ROOT_DIR/config/config.toml"    "$OVERLAY/etc/xdg/ai-os/config.toml"
cp "$SCRIPT_DIR/packages.list"       "$OVERLAY/etc/xdg/ai-os/packages.list"

# Wayfire (compositor Wayland)
cp "$SCRIPT_DIR/wayfire.ini"         "$OVERLAY/etc/wayfire/wayfire.ini" 2>/dev/null || true

# ── 3. Genera firstrun.sh ────────────────────────────────────────────────────
echo
echo "▶ 3/4  Generazione firstrun.sh"

cat > "$OUT_DIR/firstrun.sh" << 'FIRSTRUN'
#!/bin/bash
# ============================================================
#  AI-OS — Script di primo avvio (eseguito da rpi-imager / boot)
#  Viene copiato su /boot/firstrun.sh e lanciato una sola volta.
# ============================================================
set -e
exec > /var/log/ai-os-setup.log 2>&1

echo "[ai-os-setup] Avvio primo setup — $(date)"

# ── Pacchetti di sistema ─────────────────────────────────────
PKGS=$(grep -v '^#' /etc/xdg/ai-os/packages.list | tr '\n' ' ')
apt-get update -qq
apt-get install -y $PKGS

# ── sudoers: permetti a pi di usare apt-get senza password ──
# (necessario per aggiornamenti automatici AI-OS)
echo "pi ALL=(ALL) NOPASSWD: /usr/bin/apt-get" > /etc/sudoers.d/pi-apt
echo "pi ALL=(ALL) NOPASSWD: /usr/bin/install"  >> /etc/sudoers.d/pi-apt
echo "pi ALL=(ALL) NOPASSWD: /usr/local/bin/ai-os-update" >> /etc/sudoers.d/pi-apt
chmod 440 /etc/sudoers.d/pi-apt

# ── Configurazione Wayland/Wayfire per utente pi ─────────────
WAYFIRE_INI="/home/pi/.config/wayfire.ini"
mkdir -p "$(dirname $WAYFIRE_INI)"
if [ -f /etc/wayfire/wayfire.ini ]; then
    cp /etc/wayfire/wayfire.ini "$WAYFIRE_INI"
fi
chown pi:pi "$WAYFIRE_INI"

# ── Config AI-OS default se non esiste ──────────────────────
AIOS_CFG="/home/pi/.config/ai-os/config.toml"
if [ ! -f "$AIOS_CFG" ]; then
    mkdir -p "$(dirname $AIOS_CFG)"
    cp /etc/xdg/ai-os/config.toml "$AIOS_CFG"
    chown -R pi:pi /home/pi/.config/ai-os
fi

# ── Servizi systemd per l'utente pi ─────────────────────────
mkdir -p /home/pi/.config/systemd/user
cp /etc/systemd/user/ai-daemon.service    /home/pi/.config/systemd/user/
cp /etc/systemd/user/ai-os-update.service /home/pi/.config/systemd/user/
cp /etc/systemd/user/ai-os-update.timer   /home/pi/.config/systemd/user/
chown -R pi:pi /home/pi/.config/systemd

# Abilita i servizi (loginctl linger garantisce avvio senza login grafico)
loginctl enable-linger pi
sudo -u pi XDG_RUNTIME_DIR=/run/user/1000 systemctl --user enable ai-daemon
sudo -u pi XDG_RUNTIME_DIR=/run/user/1000 systemctl --user start  ai-daemon       || true
sudo -u pi XDG_RUNTIME_DIR=/run/user/1000 systemctl --user enable ai-os-update.timer
sudo -u pi XDG_RUNTIME_DIR=/run/user/1000 systemctl --user start  ai-os-update.timer || true

# ── Autostart ai-shell in Wayland ───────────────────────────
AUTOSTART_DIR="/home/pi/.config/wayfire"
mkdir -p "$AUTOSTART_DIR"
cat > "$AUTOSTART_DIR/autostart" << 'EOF'
[autostart]
ai-shell = /usr/local/bin/ai-shell
EOF
chown -R pi:pi "$AUTOSTART_DIR"

echo "[ai-os-setup] Setup completato — $(date)"
echo "[ai-os-setup] Versione AI-OS installata: $(cat /usr/local/share/ai-os/VERSION)"
echo "[ai-os-setup] Inserisci la tua Anthropic API key in: /home/pi/.config/ai-os/config.toml"

# Rimuovi questo script per non rieseguirlo
rm -f /boot/firstrun.sh
sed -i 's| systemd.run.*||g' /boot/cmdline.txt || true
FIRSTRUN

chmod +x "$OUT_DIR/firstrun.sh"

# ── 4. Crea archivio e checksum ──────────────────────────────────────────────
echo
echo "▶ 4/4  Creazione archivio e checksum"
(cd "$OUT_DIR" && tar -czf ai-os-overlay.tar.gz overlay/)

# sha256 per il sistema di aggiornamento (ai-os-update.sh lo verifica)
if command -v sha256sum &>/dev/null; then
    sha256sum "$OUT_DIR/ai-os-overlay.tar.gz" | awk '{print $1}' > "$OUT_DIR/ai-os-overlay.tar.gz.sha256"
else
    shasum -a 256 "$OUT_DIR/ai-os-overlay.tar.gz" | awk '{print $1}' > "$OUT_DIR/ai-os-overlay.tar.gz.sha256"
fi
ls -lh "$OUT_DIR/ai-os-overlay.tar.gz" "$OUT_DIR/ai-os-overlay.tar.gz.sha256"

echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Build completata! (versione $VERSION)"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║  Immagine SD completa:                                       ║"
echo "║    ./base-image/build-image.sh                               ║"
echo "║                                                              ║"
echo "║  Deploy su SD già con RPi OS Lite 64-bit:                   ║"
echo "║    1. Monta la partizione root della SD                      ║"
echo "║    2. tar -xzf dist/ai-os-overlay.tar.gz -C /mnt/           ║"
echo "║    3. cp dist/firstrun.sh /boot/firstrun.sh                  ║"
echo "║                                                              ║"
echo "║  Distribuzione aggiornamenti OTA:                           ║"
echo "║    Copia su un web server accessibile dal Pi:               ║"
echo "║      dist/version.txt (contiene: $VERSION)                  ║"
echo "║      dist/ai-os-overlay.tar.gz                               ║"
echo "║      dist/ai-os-overlay.tar.gz.sha256                        ║"
echo "║    Poi configura update_url in ~/.config/ai-os/config.toml   ║"
echo "╚══════════════════════════════════════════════════════════════╝"

# Genera anche version.txt nella dist per gli aggiornamenti OTA
echo "$VERSION" > "$OUT_DIR/version.txt"
