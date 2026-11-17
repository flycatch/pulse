# Pulse

Tiny tray/menu-bar app showing live network speed on macOS, Linux and Windows.
No admin rights needed; samples OS interface counters once per second.

![Pulse in the macOS menu bar](docs/menubar.png)

Also shows CPU, memory and system-disk usage as percentages (toggle in the menu).

- **macOS**: `↓ ↑` speed text plus CPU / MEM / DSK percentages in the menu bar.
- **Windows / Linux**: tray icon redrawn with down (green) / up (orange) speeds; full values in the tooltip.
- **Left-click** the tray icon: popup with 60 s graphs, session totals and per-interface rates.
  **Right-click**: menu (Show graph, Quit).

## Build

```
cargo build --release
```

Linux build deps: `libgtk-3-dev libayatana-appindicator3-dev libxdo-dev`.
GNOME also needs the AppIndicator extension to show tray icons.

`--popup` opens the popup at startup (handy for development).

## Install

macOS (Apple Silicon / Intel) and Linux x86_64:

```
curl -fsSL https://raw.githubusercontent.com/flycatch/pulse/main/install.sh | sh
pulse &
```

Installs to `~/.local/bin` (override with `PULSE_INSTALL_DIR`, pin a release with `PULSE_VERSION=v0.1.0`).
Uninstall: `rm ~/.local/bin/pulse`.

### From source (macOS)

```
cargo install --path .
pulse &
```

Binary lands in `~/.cargo/bin/pulse`.

**Start at login**: save as `~/Library/LaunchAgents/com.flycatch.pulse.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>com.flycatch.pulse</string>
  <key>ProgramArguments</key><array><string>/Users/YOU/.cargo/bin/pulse</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict></plist>
```

```
launchctl load ~/Library/LaunchAgents/com.flycatch.pulse.plist
```

Remove: `launchctl unload` the same file, delete it, `cargo uninstall pulse`.

The binary is unsigned. A copy downloaded from CI is quarantined by Gatekeeper:
right-click > Open, or `xattr -d com.apple.quarantine <file>`.

## Scope

Total and per-interface throughput only. Per-process bandwidth needs packet capture / elevated rights and is out of scope.
