#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
TSN_HOME="${TSN_HOME:-$HOME/.tsn}"
BIN_DIR="$TSN_HOME/bin"
STDLIB_DIR="$TSN_HOME/stdlib"
CACHE_DIR="$TSN_HOME/cache"

echo "[tsn] repo root: $REPO_ROOT"
echo "[tsn] install dir: $TSN_HOME"

mkdir -p "$BIN_DIR" "$STDLIB_DIR" "$CACHE_DIR"

echo "[tsn] building release binary..."
(cd "$REPO_ROOT" && cargo build --release --bin tsn --bin tsn-lsp)

EXE_SOURCE="$REPO_ROOT/target/release/tsn"
if [ ! -f "$EXE_SOURCE" ]; then
  echo "release binary not found at $EXE_SOURCE" >&2
  exit 1
fi

cp "$EXE_SOURCE" "$BIN_DIR/tsn"
chmod +x "$BIN_DIR/tsn"

LSP_SOURCE="$REPO_ROOT/target/release/tsn-lsp"
if [ ! -f "$LSP_SOURCE" ]; then
  echo "release lsp binary not found at $LSP_SOURCE" >&2
  exit 1
fi
cp "$LSP_SOURCE" "$BIN_DIR/tsn-lsp"
chmod +x "$BIN_DIR/tsn-lsp"

if [ ! -d "$REPO_ROOT/tsn-stdlib" ]; then
  echo "tsn-stdlib folder not found at $REPO_ROOT/tsn-stdlib" >&2
  exit 1
fi

rm -rf "$STDLIB_DIR"
mkdir -p "$STDLIB_DIR"
cp -R "$REPO_ROOT/tsn-stdlib/." "$STDLIB_DIR/"

PROFILE_FILE=""
if [ -n "${ZSH_VERSION:-}" ]; then
  PROFILE_FILE="$HOME/.zshrc"
else
  PROFILE_FILE="$HOME/.bashrc"
fi

{
  if ! grep -q "# TSN runtime" "$PROFILE_FILE" 2>/dev/null; then
    echo ""
    echo "# TSN runtime"
    echo "export TSN_HOME=\"$TSN_HOME\""
    echo "export TSN_STDLIB=\"$STDLIB_DIR\""
    echo "export TSN_CACHE_DIR=\"$CACHE_DIR\""
    echo "case \":\$PATH:\" in"
    echo "  *:\"$BIN_DIR\":*) ;;"
    echo "  *) export PATH=\"$BIN_DIR:\$PATH\" ;;"
    echo "esac"
  fi
} >> "$PROFILE_FILE"

echo ""
echo "[tsn] installed:"
echo "  binary : $BIN_DIR/tsn"
echo "  lsp    : $BIN_DIR/tsn-lsp"
echo "  stdlib : $STDLIB_DIR"
echo "  cache  : $CACHE_DIR"
echo ""
echo "[tsn] reload shell and verify:"
echo "  tsn doctor"
