#!/usr/bin/env bash
PREFIX="${PREFIX:-/usr/local}"
rm -f "${PREFIX}/bin/openpatience"
rm -f "${PREFIX}/share/applications/openpatience.desktop"
rm -rf "${PREFIX}/share/doc/openpatience"
echo "✓ Open Solitaire uninstalled"
