#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Chronos Engine — WASM Test Script
#
# Builds the project for wasm32-unknown-unknown using wasm-pack, copies assets
# into the www/ directory, and starts a local HTTP server for testing.
#
# Prerequisites:
#   - wasm-pack (install: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh)
#   - wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown)
#   - Python 3 (for the dev server)
#
# Usage:
#   ./wasm-test.sh [--serve | --build-only | --optimize]
#
# Flags:
#   --serve        Build + start dev server (default if no flag given)
#   --build-only   Only build, don't serve
#   --optimize     Build with wasm-opt size optimizations (requires binaryen)
#   --release      Build in release mode (much slower but smaller binary)
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# ── Parse flags ──────────────────────────────────────────────────────────────
DO_SERVE=true
DO_OPTIMIZE=false
RELEASE_FLAG=""

for arg in "$@"; do
    case "$arg" in
        --build-only) DO_SERVE=false ;;
        --optimize)   DO_OPTIMIZE=true ;;
        --release)    RELEASE_FLAG="--release" ;;
        --serve)      DO_SERVE=true ;;
        *)
            echo "Unknown flag: $arg"
            echo "Usage: $0 [--serve | --build-only | --optimize | --release]"
            exit 1
            ;;
    esac
done

echo "╔══════════════════════════════════════════════════════════╗"
echo "║     Chronos Engine — WASM Build & Test                  ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# ── Install target if missing ────────────────────────────────────────────────
if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "► Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# ── Ensure wasm-pack is available ────────────────────────────────────────────
if ! command -v wasm-pack &>/dev/null; then
    echo "✗ wasm-pack not found. Install it:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# ── Clean previous build ─────────────────────────────────────────────────────
echo "► Cleaning previous WASM build..."
rm -rf www/pkg

# ── Build ────────────────────────────────────────────────────────────────────
echo "► Building Chronos Engine for WASM..."
WASM_PACK_FLAGS="--target web --features web --out-name chronos_engine --out-dir www/pkg"

if [ -n "$RELEASE_FLAG" ]; then
    echo "  Mode: release"
    WASM_PACK_FLAGS="$WASM_PACK_FLAGS --mode release"
else
    echo "  Mode: debug"
fi

# Optimize for size with RUSTFLAGS on release builds
if [ -n "$RELEASE_FLAG" ]; then
    RUSTFLAGS="-C panic=abort -C opt-level=z" \
    wasm-pack build $WASM_PACK_FLAGS
else
    wasm-pack build $WASM_PACK_FLAGS
fi

echo "  ✓ Build complete!"

# ── Optional: wasm-opt size optimization ─────────────────────────────────────
if [ "$DO_OPTIMIZE" = true ] && command -v wasm-opt &>/dev/null; then
    echo "► Running wasm-opt -Oz (size optimization)..."
    WASM_FILE="www/pkg/chronos_engine_bg.wasm"
    if [ -f "$WASM_FILE" ]; then
        wasm-opt -Oz -o "$WASM_FILE" "$WASM_FILE"
        echo "  ✓ wasm-opt complete"
    else
        echo "  ⚠ WASM file not found at $WASM_FILE"
    fi
elif [ "$DO_OPTIMIZE" = true ]; then
    echo "  ⚠ wasm-opt not found. Install binaryen:"
    echo "    apt install binaryen  # Debian/Ubuntu"
    echo "    brew install binaryen # macOS"
fi

# ── Report size ──────────────────────────────────────────────────────────────
WASM_FILE="www/pkg/chronos_engine_bg.wasm"
if [ -f "$WASM_FILE" ]; then
    SIZE_KB=$(du -k "$WASM_FILE" | cut -f1)
    echo "  WASM binary size: ${SIZE_KB} KB"
fi

# ── Optional: Start dev server ───────────────────────────────────────────────
if [ "$DO_SERVE" = true ]; then
    PORT="${PORT:-8000}"
    echo ""
    echo "► Starting dev server on http://localhost:$PORT"
    echo "  Press Ctrl+C to stop"
    echo ""
    cd www
    python3 -m http.server "$PORT"
fi
