#!/usr/bin/env bash
# ============================================================
#  AI-OS — Crea immagine .img flashabile per Raspberry Pi 4
#
#  Partendo da Raspberry Pi OS Lite 64-bit ufficiale,
#  aggiunge binari, configurazione e firstrun.sh, producendo
#  un file .img.xz pronto per rpi-imager o balenaEtcher.
#
#  Prerequisiti (Linux o macOS con Docker):
#    - curl, xz, sha256sum (o shasum su macOS)
#    - sudo (per mount/losetup su Linux)
#    - Docker (modalità alternativa senza sudo)
#
#  Uso:
#    ./base-image/build-image.sh [--no-download]
#
#  Flag:
#    --no-download  Non ri-scarica l'immagine base se già presente
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$ROOT_DIR/dist"
CACHE_DIR="$ROOT_DIR/.image-cache"

# ── Versione immagine base ───────────────────────────────────────────────────
RPI_OS_URL="https://downloads.raspberrypi.com/raspios_lite_arm64/images/raspios_lite_arm64-2024-11-19/2024-11-19-raspios-bookworm-arm64-lite.img.xz"
RPI_OS_IMG="2024-11-19-raspios-bookworm-arm64-lite.img"
RPI_OS_XZ="${RPI_OS_IMG}.xz"

NO_DOWNLOAD=0
for arg in "$@"; do [[ "$arg" == "--no-download" ]] && NO_DOWNLOAD=1; done

echo "╔══════════════════════════════════════════════════════╗"
echo "║   AI-OS — Crea immagine flashabile per RPi 4         ║"
echo "╚══════════════════════════════════════════════════════╝"

mkdir -p "$OUT_DIR" "$CACHE_DIR"

# ── Verifica prerequisiti ────────────────────────────────────────────────────
check_cmd() { command -v "$1" &>/dev/null || { echo "✗ '$1' non trovato. Installalo e riprova."; exit 1; }; }
check_cmd curl
check_cmd xz

# Determina piattaforma per operazioni di mount
IS_MACOS=0
[[ "$(uname)" == "Darwin" ]] && IS_MACOS=1

if [[ $IS_MACOS -eq 0 ]]; then
    check_cmd losetup
    check_cmd mount
fi

# ── 1. Cross-compila (se non già fatto) ──────────────────────────────────────
echo
echo "▶ 1/6  Compilazione Rust"
"$SCRIPT_DIR/build.sh"

# ── 2. Download immagine base ────────────────────────────────────────────────
echo
echo "▶ 2/6  Immagine base Raspberry Pi OS Lite 64-bit"
XZ_PATH="$CACHE_DIR/$RPI_OS_XZ"
IMG_PATH="$CACHE_DIR/$RPI_OS_IMG"

if [[ $NO_DOWNLOAD -eq 0 ]] || [[ ! -f "$XZ_PATH" ]]; then
    echo "    Download: $RPI_OS_URL"
    curl -L --progress-bar -o "$XZ_PATH" "$RPI_OS_URL"
else
    echo "    Cache trovata: $XZ_PATH"
fi

if [[ ! -f "$IMG_PATH" ]]; then
    echo "    Decompressione…"
    xz --decompress --keep --force "$XZ_PATH"
    mv "$CACHE_DIR/${RPI_OS_XZ%.xz}" "$IMG_PATH" 2>/dev/null || true
fi

# ── 3. Copia immagine di lavoro ───────────────────────────────────────────────
echo
echo "▶ 3/6  Preparazione immagine di lavoro"
WORK_IMG="$OUT_DIR/ai-os.img"
cp "$IMG_PATH" "$WORK_IMG"

# ── 4. Espandi immagine (+ 400 MB per binari e pacchetti) ────────────────────
echo
echo "▶ 4/6  Espansione immagine (+400 MB)"
if [[ $IS_MACOS -eq 1 ]]; then
    # macOS: hdiutil
    hdiutil resize -size +400m "$WORK_IMG"
else
    # Linux: truncate + resize2fs dopo mount
    truncate -s +400M "$WORK_IMG"
fi

# ── 5. Inietta overlay nella partizione root ─────────────────────────────────
echo
echo "▶ 5/6  Iniezione overlay nella partizione root"

OVERLAY_DIR="$OUT_DIR/overlay"

if [[ $IS_MACOS -eq 1 ]]; then
    echo
    echo "  ⚠  Su macOS il mount diretto di ext4 non è supportato nativamente."
    echo "     Usa una delle alternative:"
    echo
    echo "  A) Docker (consigliato su macOS):"
    echo "     docker run --rm --privileged \\"
    echo "       -v $OUT_DIR:/out \\"
    echo "       ubuntu:24.04 bash -s << 'EOF'"
    echo "     apt-get install -y kpartx rsync"
    echo "     kpartx -av /out/ai-os.img"
    echo "     mount /dev/mapper/loop0p2 /mnt"
    echo "     rsync -a /out/overlay/ /mnt/"
    echo "     cp /out/firstrun.sh /mnt/boot/"
    echo "     umount /mnt && kpartx -dv /out/ai-os.img"
    echo "     EOF"
    echo
    echo "  B) Usa rpi-imager con il file overlay manuale (vedi README)."
    echo
else
    # Linux: losetup + kpartx
    if ! command -v kpartx &>/dev/null; then
        sudo apt-get install -y kpartx rsync 2>/dev/null || true
    fi

    LOOP=$(sudo losetup --find --show --partscan "$WORK_IMG")
    echo "    Loop device: $LOOP"
    sleep 1

    MOUNT_DIR=$(mktemp -d)
    ROOT_PART="${LOOP}p2"
    BOOT_PART="${LOOP}p1"

    sudo mount "$ROOT_PART" "$MOUNT_DIR"

    # Ridimensiona ext4 al nuovo spazio
    sudo resize2fs "$ROOT_PART" 2>/dev/null || true

    # Copia overlay
    sudo rsync -a "$OVERLAY_DIR/" "$MOUNT_DIR/"

    # Copia firstrun.sh nella partizione boot
    BOOT_DIR=$(mktemp -d)
    sudo mount "$BOOT_PART" "$BOOT_DIR"
    sudo cp "$OUT_DIR/firstrun.sh" "$BOOT_DIR/firstrun.sh"

    # Aggiungi hook firstrun al cmdline.txt
    if ! grep -q "firstrun" "$BOOT_DIR/cmdline.txt" 2>/dev/null; then
        sudo sed -i \
            's/$/ systemd.run=\/boot\/firstrun.sh systemd.run_success_action=reboot systemd.unit=kernel-command-line.target/' \
            "$BOOT_DIR/cmdline.txt"
    fi

    sudo umount "$BOOT_DIR" && rmdir "$BOOT_DIR"
    sudo umount "$MOUNT_DIR" && rmdir "$MOUNT_DIR"
    sudo losetup -d "$LOOP"
fi

# ── 6. Comprimi immagine finale ───────────────────────────────────────────────
echo
echo "▶ 6/6  Compressione immagine finale"
IMG_OUT="$OUT_DIR/ai-os-rpi4.img.xz"
xz --compress --threads=0 --keep --force "$WORK_IMG"
mv "${WORK_IMG}.xz" "$IMG_OUT"
ls -lh "$IMG_OUT"

echo
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║  Immagine pronta: dist/ai-os-rpi4.img.xz                        ║"
echo "╠══════════════════════════════════════════════════════════════════╣"
echo "║  Come flashare:                                                  ║"
echo "║    1. Apri Raspberry Pi Imager                                   ║"
echo "║    2. Scegli OS → Use Custom → seleziona ai-os-rpi4.img.xz      ║"
echo "║    3. Scegli la micro SD                                         ║"
echo "║    4. Clicca Write                                               ║"
echo "║                                                                  ║"
echo "║  Al primo avvio:                                                 ║"
echo "║    • firstrun.sh installa pacchetti (~5-10 min con internet)     ║"
echo "║    • Il daemon si avvia automaticamente come servizio systemd    ║"
echo "║    • Inserisci la tua API key Anthropic in:                      ║"
echo "║        ~/.config/ai-os/config.toml                               ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
