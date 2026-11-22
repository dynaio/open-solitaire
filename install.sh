#!/usr/bin/env bash
set -euo pipefail

BINARY="openpatience"
PREFIX="${PREFIX:-/usr/local}"
DESTDIR="${DESTDIR:-}"

echo "==> Building OpenPatience (release)..."
cargo build --release

echo "==> Installing binary..."
install -Dm755 "target/release/$BINARY" "${DESTDIR}${PREFIX}/bin/$BINARY"

echo "==> Installing desktop entry..."
install -Dm644 "openpatience.desktop" "${DESTDIR}${PREFIX}/share/applications/openpatience.desktop"

echo "==> Installing README..."
install -Dm644 "README.md" "${DESTDIR}${PREFIX}/share/doc/openpatience/README.md"

echo ""
echo "✓ Open Solitaire installed to ${DESTDIR}${PREFIX}/bin/$BINARY"
echo "  Run: openpatience"
