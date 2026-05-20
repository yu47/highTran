# HighTran

Secure peer-to-peer file transfer relayed via **Lark Suite (飞书) Drive API**.

Files are encrypted end-to-end with AES-256-GCM before upload. The relay server (Lark Drive) only sees encrypted blobs — the encryption key is derived from a short pickup code exchanged out-of-band.

## Features

- **Send** — Select a file, get a 6-character pickup code
- **Receive** — Enter the pickup code and a save directory to download
- **Speed Test** — Measure upload/download throughput via Lark Drive (1/8/16 MB)
- **Dark mode** — Toggle between light and dark themes
- **i18n** — Chinese and English interfaces
- **Encrypted relay** — AES-256-GCM with key derived from the pickup code (MD5)

## Architecture

```
Sender                    Lark Drive                    Receiver
  │                          │                            │
  │──── meta.enc ──────────►│                            │
  │                          │◄──── start.enc ───────────│
  │──── chunk_N.enc ───────►│                            │
  │──── complete.enc ──────►│◄──── chunk_N.enc ─────────│
  │                          │                            │
```

- All files are encrypted client-side before upload
- The pickup code serves as both the rendezvous identifier and the key seed
- Relies on Lark Drive's folder/file API as the transport layer

## Build

### Prerequisites

- [Rust](https://rustup.rs/) (MSVC toolchain on Windows)
- [Node.js](https://nodejs.org/) 18+
- [Tauri v2 CLI](https://v2.tauri.app/start/prerequisites/)

### Commands

```bash
# Install frontend dependencies
npm install

# Development mode (hot-reload)
npm run tauri dev

# Production build (MSI + NSIS on Windows)
npm run tauri build
```

The built installer is output to:

```
src-tauri/target/release/bundle/msi/          # Windows installer (MSI)
src-tauri/target/release/bundle/nsis/         # Windows installer (NSIS)
```

## Configuration

Credentials are configured in the app's Settings panel (gear icon). The default credentials are hardcoded for development:

- **App ID**: `cli_aa871fad79f8de15`
- **App Secret**: `FMVDzOY6TVErA94tzzuFHeDlnigRui72`

Settings are persisted in `localStorage` under the key `ft-lark-config`.

## Tech Stack

| Layer       | Technology                     |
|-------------|--------------------------------|
| GUI         | Tauri v2 + Vue 3 + TypeScript |
| Backend     | Rust (reqwest, tokio)          |
| Encryption  | AES-256-GCM (aes-gcm crate)   |
| Key Derivation | MD5 (md-5 crate)            |
| Transport   | Lark Suite Drive REST API      |
| Bundling    | MSI / NSIS                     |

## License

MIT
