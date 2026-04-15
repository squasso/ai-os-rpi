# AI-OS per Raspberry Pi 4

Desktop environment per Raspberry Pi 4 con Claude AI integrato al centro del sistema. Scritto in Rust, basato su Wayland/Wayfire.

![AI-OS screenshot placeholder](assets/screenshot.png)

## Cosa fa

- **Assistente AI nativo** — Claude risponde e agisce direttamente sul sistema (apre app, scrive file, esegue comandi shell, pubblica articoli)
- **Routing intelligente dei modelli** — Haiku per comandi veloci, Sonnet per scrittura e analisi, Opus per ragionamenti complessi
- **Budget controllato** — limiti giornalieri e mensili sull'API Anthropic, con avvisi e blocco automatico
- **Blog publishing** — WordPress e Ghost direttamente dall'assistente
- **Aggiornamenti OTA** — il sistema si aggiorna da un endpoint configurabile senza reflashare la SD

---

## Requisiti

| Componente | Minimo |
|---|---|
| Hardware | Raspberry Pi 4 (2 GB RAM consigliati) |
| OS base | Raspberry Pi OS Lite 64-bit (Bookworm) |
| Connessione | Internet (per API Anthropic) |
| API key | [console.anthropic.com](https://console.anthropic.com) |

---

## Installazione rapida (immagine pre-compilata)

### 1. Scarica l'immagine

Dalla [pagina Releases](../../releases/latest) scarica:
- `ai-os-rpi4.img.xz` — immagine completa

### 2. Flasha la SD

Usa **Raspberry Pi Imager**:
1. *Choose OS* → *Use Custom* → seleziona `ai-os-rpi4.img.xz`
2. *Choose Storage* → seleziona la micro SD (≥ 16 GB)
3. Clicca l'icona ⚙️ e configura: username `pi`, password, Wi-Fi
4. *Write*

### 3. Primo avvio

Il Pi installerà automaticamente i pacchetti (~5-10 minuti con connessione internet). Al termine:

```
/home/pi/.config/ai-os/config.toml
```

Apri questo file e inserisci la tua API key Anthropic:

```toml
api_key = "sk-ant-..."
```

Riavvia il daemon:
```bash
systemctl --user restart ai-daemon
```

**In alternativa**, al primo avvio dello shell apparirà automaticamente una schermata di configurazione guidata.

---

## Build da sorgente

### Prerequisiti

```bash
# Installa Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Tool per cross-compilation
cargo install cross

# Su Linux: cross-linker
sudo apt install gcc-aarch64-linux-gnu

# Su macOS:
brew install FiloSottile/musl-cross/musl-cross
```

### Build dei binari

```bash
git clone https://github.com/TUO-USERNAME/ai-os-rpi
cd ai-os-rpi

# Compila per RPi 4 (aarch64)
cross build --release --target aarch64-unknown-linux-gnu

# Oppure per test locale (x86_64)
cargo build --release
```

### Crea overlay e immagine

```bash
# Solo overlay (per deploy manuale su SD già pronta)
./base-image/build.sh

# Immagine completa flashabile (richiede sudo su Linux)
./base-image/build-image.sh
```

I file vengono prodotti in `dist/`:
```
dist/
  ai-os-overlay.tar.gz           ← binari + config
  ai-os-overlay.tar.gz.sha256    ← checksum integrità
  version.txt                    ← versione corrente
  firstrun.sh                    ← script primo avvio
  ai-os-rpi4.img.xz              ← (solo build-image.sh) immagine completa
```

### Deploy manuale su SD esistente

```bash
# Monta la partizione root della SD
sudo mount /dev/sdX2 /mnt

# Applica overlay
sudo tar -xzf dist/ai-os-overlay.tar.gz -C /mnt/

# Copia firstrun.sh nella partizione boot
sudo mount /dev/sdX1 /boot
sudo cp dist/firstrun.sh /boot/firstrun.sh

sudo umount /boot /mnt
```

---

## Configurazione

### File di configurazione principale

`~/.config/ai-os/config.toml`:

```toml
api_key = "sk-ant-..."   # chiave Anthropic (obbligatorio)

[budget]
monthly_limit_usd    = 10.0   # blocca le richieste oltre questo limite
daily_soft_limit_usd = 0.50   # avviso giornaliero
alert_at_percent     = 80     # avviso quando si raggiunge l'80%

[update]
update_url = ""    # URL endpoint aggiornamenti (vedi sezione OTA)
auto_check = true  # controlla ogni domenica alle 03:00

# [wordpress]
# url          = "https://tuoblog.com"
# username     = "admin"
# app_password = "xxxx xxxx xxxx xxxx xxxx xxxx"

# [ghost]
# url       = "https://tuoblog.ghost.io"
# admin_key = "id:hex_secret"
```

Tutte le impostazioni sono modificabili anche dalla UI: icona ⚙️ nel pannello laterale → *Impostazioni*.

### Icone personalizzate

Salva immagini PNG in `~/.config/ai-os/icons/<nome-app>.png` (es. `chromium.png`, `gimp.png`). Il sistema le carica automaticamente; se assenti usa icone lettera colorate.

---

## Aggiornamenti OTA

### Configurare la sorgente

L'URL deve esporre tre file statici:

```
https://tuo-server.com/ai-os/
  version.txt                    ← es. "0.2.0"
  ai-os-overlay.tar.gz           ← binari compilati per aarch64
  ai-os-overlay.tar.gz.sha256    ← sha256 dell'archivio
```

Se il progetto è su GitHub, usa direttamente i GitHub Releases:
```toml
[update]
update_url = "https://github.com/TUO-USERNAME/ai-os-rpi/releases/latest/download/"
```

### Aggiornare il sistema

**Via assistente AI** (modo più semplice):
> "Aggiorna il sistema"
> "Controlla se ci sono aggiornamenti"
> "Aggiorna i pacchetti di sistema"

**Via impostazioni**: ⚙️ → *Aggiornamenti* → pulsanti Controlla / Aggiorna

**Via terminale**:
```bash
ai-os-update --check           # controlla versione disponibile
ai-os-update --apply           # installa nuovi binari
ai-os-update --apply-packages  # aggiorna pacchetti apt
```

Il timer systemd controlla automaticamente ogni domenica alle 03:00 + 10 minuti dopo ogni avvio.

---

## Architettura

```
ai-shell (GUI)          ai-daemon (backend)
  egui/eframe   ←IPC→   tokio + reqwest
  Wayland               Anthropic API
  Unix socket           SQLite (memoria)
                        WordPress/Ghost
```

### Routing modelli AI

| Trigger | Modello | Max token | Costo |
|---|---|---|---|
| Comando semplice, azione rapida | Haiku | 512 | $ |
| Scrittura, codice, analisi, blog | Sonnet | 4096 | $$ |
| Ragionamento strategico, architetture, prompt > 800 char | Opus | 8192 | $$$ |

### Struttura repository

```
crates/
  common/         tipi condivisi, protocollo IPC
  ai-daemon/      backend: API, tool engine, budget, memoria
  ai-shell/       frontend: GUI egui, panel, menubar, settings
  blog-publisher/ libreria WordPress + Ghost
scripts/
  ai-os-update.sh script di aggiornamento OTA
systemd/
  ai-daemon.service
  ai-os-update.service + .timer
base-image/
  build.sh        crea overlay + firstrun.sh
  build-image.sh  crea immagine .img.xz flashabile
  wayfire.ini     configurazione compositor Wayland
config/
  config.toml     template configurazione default
```

---

## Comandi utili sul Pi

```bash
# Stato servizi
systemctl --user status ai-daemon
systemctl --user status ai-os-update.timer

# Log in tempo reale
journalctl --user -u ai-daemon -f

# Riavvio daemon (dopo modifica config)
systemctl --user restart ai-daemon

# Aggiornamento manuale
ai-os-update --check
ai-os-update --apply

# Versione installata
cat /usr/local/share/ai-os/VERSION
```

---

## Licenza

MIT
