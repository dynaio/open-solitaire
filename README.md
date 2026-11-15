# Open Solitaire

**A beautiful, offline-first, open-source Solitaire suite for Linux.**

Native GTK4 + Cairo rendering · Deterministic deals · Zero network code

## Features

- 🎴 **Multiple game modes** — Klondike (draw 1 & 3), FreeCell, Spider (WIP)
- ⚡ **Native GTK4** — real Linux app, not a webview. Adwaita header bar, Wayland-first
- 🎨 **Cairo-rendered cards** — vector suit shapes, real pip layouts, smooth shadows
- 🔒 **Offline forever** — zero network code, no telemetry, no backdoors
- 🎯 **Deterministic deals** — every game has a shareable seed number
- ⌨️ **Keyboard & mouse** — `U` undo, `N` new, `D` draw, `F` auto-foundation

## Screenshots

*(coming soon)*

## Build from source

### Requirements
- Rust 1.78+ (`rustup` recommended)
- `libgtk-4-dev` and `libadwaita-1-dev`

### Ubuntu / Debian
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
git clone https://github.com/abdelkader/open-solitaire
cd open-solitaire
cargo build --release
./target/release/openpatience

