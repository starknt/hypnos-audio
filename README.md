# Hypnos Audio

A lightweight Windows background utility that automatically mutes system audio when a Bluetooth headset is disconnected, and restores the previous volume state when reconnected.

## Features

### Automatic Headset Detection
- Monitors Windows audio endpoint changes via the MMDevice API
- Detects headphone/headset devices by their form factor (`Headphones` or `Headset`)
- Distinguishes between device connect (`ACTIVE`) and disconnect (`NOTPRESENT` / `UNPLUGGED`) events

### Smart Mute & Restore
- **On disconnect**: Immediately saves the current audio state (mute status + master volume), then mutes the system after a 500 ms debounce
- **On reconnect**: Restores the previously saved headphone volume and mute state
- Uses an event generation counter to prevent stale debounce tasks from overriding reconnect actions during rapid plug/unplug cycles

### Toast Notifications
- Displays Windows toast notifications on headset connect/disconnect
- Notifications include the app icon (`appLogoOverride`) for visual clarity
- Same-tag notifications replace previous ones to avoid stacking in Action Center

### System Tray
- Runs silently in the system tray with a status icon
- Tray menu options:
  - **Check for updates**: Manually trigger update check
  - **Launch on startup**: Toggle Windows startup registry entry
  - **Exit**: Gracefully shut down the application

### Auto-Update
- Integrated with Velopack for silent automatic updates
- Checks for new releases from GitHub on startup
- Downloads and installs updates in the background, then restarts

### Single Instance
- Uses a named mutex to ensure only one instance runs at a time
- Launching a second instance exits immediately

## Architecture

| Module | Responsibility |
|--------|---------------|
| `main.rs` | Entry point, single-instance guard, tokio runtime, module wiring |
| `bluetooth.rs` | Audio endpoint notification client, event loop with debounce logic |
| `audio.rs` | Windows Core Audio API wrapper (mute, volume get/set) |
| `state.rs` | In-memory audio state snapshot storage |
| `notifications.rs` | Windows toast notification builder with icon support |
| `tray.rs` | System tray icon and menu using `tray-icon` + `winit` |
| `startup.rs` | Windows registry read/write for startup toggle |
| `updater.rs` | Velopack update check / download / apply |

## Build

Requires Rust 1.85+ and Windows.

```bash
cargo build --release
```

The release binary is optimized for size (~2.2 MiB) with LTO, single codegen unit, and `panic = abort`.

## Configuration

Set the environment variable `HYPNOS_GITHUB_REPO` (e.g., `owner/repo`) to enable automatic update checks. If not set, update checks are skipped.

## License

MIT
