# Overmax

[한국어](README.md) | [English](README.en.md)

An overlay tool that shows unofficial V-Archive-based difficulty ratings in real time on the DJMAX RESPECT V song-select screen.

> **🚀 Native Rust app**: Overmax is built as a native Rust application for a lightweight, fast experience.
> - **Lightweight and fast**: minimal memory footprint and executable size, with strong overall runtime performance.
> - **Minimal external dependencies**: no heavy OpenCV dependency — uses pure-Rust HOG jacket image matching and Windows' built-in OCR instead.
> - **Fully backward compatible**: works with existing users' settings (`settings.json`) and local records (`record.db`), and preserves the existing portable environment as-is.

---

## User Guide

### What does it do?

It displays the **unofficial V-Archive difficulty** and a **list of similarly-difficulty recommendations** for the currently selected song, right next to the game screen.

- Shows the unofficial difficulty of the currently selected song for each button mode (NM/HD/MX/SC)
- **V-Archive record sync**: imports your V-Archive play records, and can register locally collected records to V-Archive
- **Real-time Rate / Max Combo capture**: automatically detects and saves new records locally as you play (with quick V-Archive upload support when a new best is detected in real time)
- **Similar-difficulty recommendations**: recommends other patterns with a similar difficulty to the current one (sorted by lowest Rate first, then unplayed)
- **Lite Mode**: hides non-essential elements like the recommendation list, showing only essential info (song info, real-time Rate, etc.) in a compact layout (roughly 60px tall)
- **Real-time new-record and quick-upload notifications**: if the Rate detected during play is higher than your existing V-Archive record, an **upload button (⬆)** lights up in the overlay header so you can easily sync your latest record to V-Archive.

The app never reads process memory or modifies game files — it works purely by **window tracking + screen capture**.

### Installation

1. Download the latest `overmax.zip` from [Releases](https://github.com/orphera/overmax/releases).
2. Unzip and run `overmax.exe`.
3. Launch DJMAX RESPECT V while it's running and detection starts automatically.

> **Auto-update**: on startup, the app automatically checks for a newer version and for song DB (`image_index.db`) updates, and applies them.

### Requirements

- Windows 10 1809 or later (64-bit) — Windows OCR is required
- DJMAX RESPECT V (Steam)
- An active internet connection while running (for downloading V-Archive data and checking for app/DB updates)

> ⚠️ **Important: game display settings**
> * **Borderless fullscreen (windowed fullscreen) is recommended**: to have the overlay window display correctly on top of the game while playing, set the game's display option to **"Borderless Fullscreen"**.
> * **If using exclusive fullscreen**: running the game in regular **"Fullscreen"** mode causes the overlay to render behind the game instead of on top of it, due to Windows OS and the game's anti-cheat (XIGNCODE3) restrictions. If you must use exclusive fullscreen, drag the overlay window onto a **secondary monitor** in a dual-monitor setup and use it there instead.

> **Note**: the overlay UI language can be switched to English from Settings (Korean is the default).

### Settings

- Click the **gear button (⚙)** in the overlay header to open the settings window.
- From the settings window you can adjust **overlay size (S / M / L / XL)** and **opacity**.
- The overlay uses egui's native drag support, so you can smoothly move it anywhere with the mouse; its position is saved automatically.
- **Lite Mode** can be enabled from the settings window. While Lite Mode is active, accidental drag movement is blocked, and the overlay automatically snaps to and locks onto the configured screen corner (top-left, top-right, bottom-left, bottom-right) without jitter.

---

## Developer Guide

### Build & run

```bash
# Requires Rust (rustup)
cargo build --release -p overmax-app
./target/release/overmax.exe
```

### Project structure (Rust)

- `rust/overmax_app`: main application (egui/winit-based UI and event loop)
- `rust/overmax_core`: core state model and shared logic
- `rust/overmax_data`: settings, DB (SQLite), V-Archive API integration
- `rust/overmax_cv`: core image-processing algorithms (HOG, OCR preprocessing, etc.)

### Build & release scripts

- `scripts/package-rust.ps1`: automates the full build and produces the release `overmax.zip` and `release_manifest.json` (kept in the same format as the existing release layout)

---

## Data source

- [V-Archive](https://v-archive.net)

---

## License

MIT
