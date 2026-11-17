# network-monitor

Tiny tray/menu-bar app showing live network speed on macOS, Linux and Windows.
No admin rights needed; samples OS interface counters once per second.

- **macOS**: `↓ ↑` speed text in the menu bar.
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

## Scope

Total and per-interface throughput only. Per-process bandwidth needs packet capture / elevated rights and is out of scope.
