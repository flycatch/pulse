#!/bin/sh
# Install Pulse: curl -fsSL https://raw.githubusercontent.com/flycatch/pulse/main/install.sh | sh
# Env: PULSE_VERSION=v0.1.0 to pin a release, PULSE_INSTALL_DIR to change target dir.
set -eu

REPO="flycatch/pulse"
DIR="${PULSE_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)               target=aarch64-apple-darwin ;;
  Darwin-x86_64)              target=x86_64-apple-darwin ;;
  Linux-x86_64|Linux-amd64)   target=x86_64-unknown-linux-gnu ;;
  *) echo "pulse: unsupported platform $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

if [ -n "${PULSE_VERSION:-}" ]; then
  base="https://github.com/$REPO/releases/download/$PULSE_VERSION"
else
  base="https://github.com/$REPO/releases/latest/download"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
file="pulse-$target.tar.gz"

echo "Downloading $file"
curl -fsSL "$base/$file" -o "$tmp/$file"
curl -fsSL "$base/$file.sha256" -o "$tmp/$file.sha256"

echo "Verifying checksum"
want="$(cut -d' ' -f1 "$tmp/$file.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "$tmp/$file" | cut -d' ' -f1)"
else
  got="$(shasum -a 256 "$tmp/$file" | cut -d' ' -f1)"
fi
[ "$want" = "$got" ] || { echo "pulse: checksum mismatch" >&2; exit 1; }

mkdir -p "$DIR"
tar -xzf "$tmp/$file" -C "$tmp"
install -m 755 "$tmp/pulse" "$DIR/pulse"
[ "$(uname -s)" = Darwin ] && xattr -d com.apple.quarantine "$DIR/pulse" 2>/dev/null || true

echo "Installed $DIR/pulse"
case ":$PATH:" in
  *":$DIR:"*) ;;
  *) echo "Note: $DIR is not on your PATH." ;;
esac
if [ "$(uname -s)" = Linux ]; then
  echo "Linux needs libgtk-3, libayatana-appindicator3 and libxdo; GNOME also needs the AppIndicator extension."
fi
echo "Run: pulse &"
