#!/usr/bin/env bash
# ============================================================
#  AI-OS Update Script
#  Aggiorna i binari di AI-OS scaricando dall'endpoint
#  configurato in ~/.config/ai-os/config.toml.
#
#  Uso:
#    ai-os-update --check           → solo controllo versione
#    ai-os-update --apply           → scarica e installa binari
#    ai-os-update --apply-packages  → aggiorna pacchetti apt
#    ai-os-update                   → check + apply se disponibile
#
#  Output JSON su stdout (per integrazione con Claude/tool engine):
#    { "status": "up_to_date"|"update_available"|"updated"|"error",
#      "current_version": "...", "available_version": "...",
#      "message": "..." }
# ============================================================
set -euo pipefail

VERSION_FILE="/usr/local/share/ai-os/VERSION"
CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/ai-os/config.toml"
TMP_DIR=""

# ── Utility ──────────────────────────────────────────────────────────────────

log()  { echo "[ai-os-update] $*" >&2; }
json() { printf '%s\n' "$1"; }

cleanup() {
    [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]] && rm -rf "$TMP_DIR"
}
trap cleanup EXIT

current_version() {
    cat "$VERSION_FILE" 2>/dev/null | tr -d '[:space:]' || echo "unknown"
}

# Legge il valore di una chiave da config.toml (semplice regex, no parser TOML)
read_config() {
    local key="$1"
    grep -E "^${key}\s*=" "$CONFIG_FILE" 2>/dev/null \
        | sed 's/.*=\s*"\(.*\)".*/\1/' \
        | head -1 \
        | tr -d '[:space:]'
}

# ── Parsing argomenti ─────────────────────────────────────────────────────────

MODE="auto"  # auto = check + apply se disponibile
case "${1:-}" in
    --check)           MODE="check"    ;;
    --apply)           MODE="apply"    ;;
    --apply-packages)  MODE="packages" ;;
    --help|-h)
        sed -n '3,20p' "$0"
        exit 0
        ;;
esac

# ── Leggi configurazione ──────────────────────────────────────────────────────

UPDATE_URL=$(read_config "update_url")

if [[ -z "$UPDATE_URL" ]]; then
    json "{\"status\":\"error\",\"current_version\":\"$(current_version)\",\"available_version\":\"\",\"message\":\"update_url non configurato in ~/.config/ai-os/config.toml. Aggiungi: update_url = \\\"https://tuo-server/ai-os/\\\"\"}"
    exit 1
fi

UPDATE_URL="${UPDATE_URL%/}"   # rimuovi trailing slash

# ── Modalità: aggiornamento pacchetti apt ─────────────────────────────────────

if [[ "$MODE" == "packages" ]]; then
    log "Aggiornamento pacchetti di sistema…"
    OUTPUT=$(sudo apt-get update -qq 2>&1 && sudo apt-get upgrade -y 2>&1 || true)
    json "{\"status\":\"updated\",\"current_version\":\"$(current_version)\",\"available_version\":\"\",\"message\":\"Pacchetti aggiornati. $OUTPUT\"}"
    exit 0
fi

# ── Controllo versione disponibile ────────────────────────────────────────────

CURRENT=$(current_version)
log "Versione corrente: $CURRENT"
log "Controllo aggiornamenti su: $UPDATE_URL/version.txt"

AVAILABLE=$(curl -fsSL --connect-timeout 10 "$UPDATE_URL/version.txt" 2>/dev/null | tr -d '[:space:]' || echo "")

if [[ -z "$AVAILABLE" ]]; then
    json "{\"status\":\"error\",\"current_version\":\"$CURRENT\",\"available_version\":\"\",\"message\":\"Impossibile raggiungere $UPDATE_URL/version.txt — controlla la connessione e l'URL.\"}"
    exit 1
fi

log "Versione disponibile: $AVAILABLE"

# Confronto versioni (semver semplice: major.minor.patch)
version_gt() {
    [ "$(printf '%s\n' "$@" | sort -V | head -1)" != "$1" ]
}

if [[ "$MODE" == "check" ]]; then
    if version_gt "$AVAILABLE" "$CURRENT"; then
        json "{\"status\":\"update_available\",\"current_version\":\"$CURRENT\",\"available_version\":\"$AVAILABLE\",\"message\":\"Aggiornamento disponibile: $CURRENT → $AVAILABLE. Esegui 'ai-os-update --apply' per installare.\"}"
    else
        json "{\"status\":\"up_to_date\",\"current_version\":\"$CURRENT\",\"available_version\":\"$AVAILABLE\",\"message\":\"AI-OS è aggiornato (versione $CURRENT).\"}"
    fi
    exit 0
fi

# ── auto: esce se già aggiornato ──────────────────────────────────────────────

if [[ "$MODE" == "auto" ]] && ! version_gt "$AVAILABLE" "$CURRENT"; then
    json "{\"status\":\"up_to_date\",\"current_version\":\"$CURRENT\",\"available_version\":\"$AVAILABLE\",\"message\":\"AI-OS è già alla versione $CURRENT.\"}"
    exit 0
fi

# ── Download e verifica ───────────────────────────────────────────────────────

TMP_DIR=$(mktemp -d)
TARBALL="$TMP_DIR/ai-os-overlay.tar.gz"

log "Download: $UPDATE_URL/ai-os-overlay.tar.gz"
curl -fL --progress-bar "$UPDATE_URL/ai-os-overlay.tar.gz" -o "$TARBALL"

# Verifica sha256 se disponibile
SHA256_URL="$UPDATE_URL/ai-os-overlay.tar.gz.sha256"
if curl -fsSL --connect-timeout 5 "$SHA256_URL" -o "$TMP_DIR/checksum.sha256" 2>/dev/null; then
    log "Verifica checksum…"
    EXPECTED=$(awk '{print $1}' "$TMP_DIR/checksum.sha256")
    if command -v sha256sum &>/dev/null; then
        ACTUAL=$(sha256sum "$TARBALL" | awk '{print $1}')
    else
        ACTUAL=$(shasum -a 256 "$TARBALL" | awk '{print $1}')
    fi
    if [[ "$EXPECTED" != "$ACTUAL" ]]; then
        json "{\"status\":\"error\",\"current_version\":\"$CURRENT\",\"available_version\":\"$AVAILABLE\",\"message\":\"Checksum non valido — aggiornamento annullato.\"}"
        exit 1
    fi
    log "Checksum OK"
fi

# ── Installazione ─────────────────────────────────────────────────────────────

EXTRACT_DIR="$TMP_DIR/overlay"
mkdir -p "$EXTRACT_DIR"
tar -xzf "$TARBALL" -C "$EXTRACT_DIR" --strip-components=1

log "Installazione binari…"

# Stop daemon prima di sostituire il binario
XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
systemctl --user stop ai-daemon 2>/dev/null || true

# Copia binari (sudo necessario per /usr/local/bin)
if [[ -f "$EXTRACT_DIR/usr/local/bin/ai-daemon" ]]; then
    sudo install -m 755 "$EXTRACT_DIR/usr/local/bin/ai-daemon" /usr/local/bin/ai-daemon
    log "ai-daemon aggiornato"
fi

if [[ -f "$EXTRACT_DIR/usr/local/bin/ai-shell" ]]; then
    sudo install -m 755 "$EXTRACT_DIR/usr/local/bin/ai-shell" /usr/local/bin/ai-shell
    log "ai-shell aggiornato (attivo al prossimo avvio)"
fi

# Aggiorna VERSION
sudo mkdir -p /usr/local/share/ai-os
echo "$AVAILABLE" | sudo tee "$VERSION_FILE" > /dev/null

# Riavvia daemon
systemctl --user start ai-daemon 2>/dev/null || true
log "ai-daemon riavviato"

json "{\"status\":\"updated\",\"current_version\":\"$AVAILABLE\",\"available_version\":\"$AVAILABLE\",\"message\":\"AI-OS aggiornato con successo: $CURRENT → $AVAILABLE. Il daemon è stato riavviato. La shell verrà aggiornata al prossimo avvio.\"}"
