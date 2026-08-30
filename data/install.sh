#!/usr/bin/env bash
# Install zPad for the current user: binary, app-menu entry, and icon.
# No root required. Uninstall: remove the three installed paths below.
set -euo pipefail

prefix="${HOME}/.local"
bindir="${prefix}/bin"
desktopdir="${prefix}/share/applications"
icondir="${prefix}/share/icons/hicolor/scalable/apps"

cd "$(dirname "$0")/.."

if [[ ! -x target/release/zpad ]]; then
    echo "Building zPad (release)..."
    cargo build --release
fi

install -Dm755 target/release/zpad "${bindir}/zpad"
install -Dm644 data/zpad.desktop "${desktopdir}/zpad.desktop"
install -Dm644 data/zpad.svg "${icondir}/zpad.svg"

# Point the launcher at the installed binary even if ~/.local/bin is not on PATH.
sed -i "s|^Exec=.*|Exec=${bindir}/zpad|" "${desktopdir}/zpad.desktop"

update-desktop-database "${desktopdir}" 2>/dev/null || true
gtk-update-icon-cache -q -t -f "${prefix}/share/icons/hicolor" 2>/dev/null || true

echo "Installed:"
echo "  ${bindir}/zpad"
echo "  ${desktopdir}/zpad.desktop"
echo "  ${icondir}/zpad.svg"
echo
echo "Launch zPad from your application menu, or run: ${bindir}/zpad"
