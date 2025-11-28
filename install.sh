#!/bin/sh
set -e

REPO="Blad3Mak3r/pg_lexo"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

usage() {
  echo "Usage: sh install.sh [PGVERSION]"
  echo "Downloads the latest version of pg_lexo and installs it for PostgreSQL PGVERSION."
  echo "If PGVERSION is not provided, it will be detected automatically."
  echo "Manual example: sh install.sh 18"
  exit 1
}

# Try to auto-detect the PostgreSQL version using pg_config
detect_pg_version() {
  PG_CONFIG=$(command -v pg_config)
  if [ -z "$PG_CONFIG" ]; then
    return 1
  fi
  VERSION=$($PG_CONFIG --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
  PG_VER=$(echo "$VERSION" | cut -d. -f1)
  echo "$PG_VER"
  return 0
}

# Parse arguments: use parameter or try auto-detect
if [ $# -eq 1 ]; then
  PG_VERSION="$1"
elif [ $# -eq 0 ]; then
  PG_VERSION=$(detect_pg_version)
  if [ -z "$PG_VERSION" ]; then
    echo "Could not detect PostgreSQL version, please provide it as an argument:"
    usage
  else
    echo "Detected PostgreSQL version $PG_VERSION"
  fi
else
  usage
fi

# --- MUSL vs GLIBC check ---
if ldd --version 2>&1 | grep -qi musl; then
  echo "ERROR! Your system uses musl (e.g., Alpine Linux)."
  echo "Currently, only glibc-based systems (e.g., Ubuntu, Debian, etc.) are supported."
  exit 10
fi

# Set OS and architecture (only linux-x64 currently supported)
OS="linux"
ARCH="x64"

# Get the latest release tag from the GitHub API
echo "Fetching latest release from ${REPO}..."
LATEST=$(curl -sSL "$API_URL" | grep -oP '"tag_name":\s*"\K([^"]+)' | head -1)
if [ -z "$LATEST" ]; then
  echo "Could not fetch the latest version"
  exit 2
fi

ASSET="pg_lexo-${LATEST}-${OS}-${ARCH}-pg${PG_VERSION}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}"

TMP_DIR=$(mktemp -d)
TAR_PATH="$TMP_DIR/$ASSET"

echo "Downloading $ASSET from $DOWNLOAD_URL"
if ! curl -sSLf -o "$TAR_PATH" "$DOWNLOAD_URL"; then
  echo "Could not download the asset. Does this PostgreSQL version exist for the extension?"
  exit 3
fi

echo "Extracting..."
tar -xzvf "$TAR_PATH" -C "$TMP_DIR"

PG_CONFIG=$(command -v pg_config)
if [ -z "$PG_CONFIG" ]; then
  echo "pg_config not found in PATH"
  exit 4
fi

PG_LIBDIR=$($PG_CONFIG --pkglibdir)
PG_SHAREDIR=$($PG_CONFIG --sharedir)/extension

echo "Installing files..."
find "$TMP_DIR" -name "*.so" -exec cp -v {} "$PG_LIBDIR" \;
find "$TMP_DIR" -name "*.control" -exec cp -v {} "$PG_SHAREDIR" \;
find "$TMP_DIR" -name "*.sql" -exec cp -v {} "$PG_SHAREDIR" \;

rm -rf "$TMP_DIR"
echo "pg_lexo installed for PostgreSQL $PG_VERSION (version $LATEST)!"
echo "You may now run: CREATE EXTENSION pg_lexo; inside PostgreSQL."
