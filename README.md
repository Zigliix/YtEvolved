# YouTube Music Evolved

A lightweight YouTube Music desktop client built with [Tauri v2](https://tauri.app/).

## What it does

- Opens **music.youtube.com** in a native desktop window
- Dynamically updates the window title with the currently playing song and artist
- Uses a Chrome user-agent for full YouTube Music compatibility

## Stack

- **Tauri v2** — Rust backend + native webview
- **Vite + TypeScript** — frontend tooling
- **Rust** — window management & IPC commands

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```
