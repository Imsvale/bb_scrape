#!/usr/bin/env bash
# scripts/mac-linux/build.sh
# Packaging script for Linux and macOS

set -e

# Default parameters
OUT_DIR="artifacts"
CARGO_TOML_PATH="Cargo.toml"
EXTRA_FILES=()
BUILD_RELEASE=false

# Detect platform
OS_NAME=$(uname -s)
ARCH=$(uname -m)

case "$OS_NAME" in
  Linux*)
    PLATFORM="linux"
    BINARY_EXT=""
    ;;
  Darwin*)
    PLATFORM="macos"
    BINARY_EXT=""
    ;;
  *)
    echo "Unsupported OS: $OS_NAME" >&2
    exit 1
    ;;
esac

# Map architecture names
case "$ARCH" in
  x86_64|amd64)
    ARCH_NAME="x86_64"
    ;;
  aarch64|arm64)
    ARCH_NAME="aarch64"
    ;;
  *)
    ARCH_NAME="$ARCH"
    ;;
esac

PLATFORM_ARCH="${PLATFORM}_${ARCH_NAME}"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --cargo-toml)
      CARGO_TOML_PATH="$2"
      shift 2
      ;;
    --arch)
      PLATFORM_ARCH="$2"
      shift 2
      ;;
    --extra-file)
      EXTRA_FILES+=("$2")
      shift 2
      ;;
    --build-release)
      BUILD_RELEASE=true
      shift
      ;;
    --help|-h)
      cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --out-dir DIR         Output directory (default: artifacts)
  --cargo-toml PATH     Path to Cargo.toml (default: Cargo.toml)
  --arch ARCH          Platform architecture (default: auto-detected)
  --extra-file FILE     Additional file to include (can be used multiple times)
  --build-release       Build release binaries before packaging
  --help, -h            Show this help message

Examples:
  $0 --build-release
  $0 --out-dir releases --arch linux_arm64
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Use --help for usage information" >&2
      exit 1
      ;;
  esac
done

# Function to extract version from Cargo.toml
get_version() {
  if [[ ! -f "$CARGO_TOML_PATH" ]]; then
    echo "Cargo.toml not found at '$CARGO_TOML_PATH'" >&2
    exit 1
  fi

  local in_package=false
  while IFS= read -r line; do
    if [[ "$line" =~ ^\[package\] ]]; then
      in_package=true
      continue
    fi
    if [[ "$in_package" == true ]] && [[ "$line" =~ ^\[ ]]; then
      break
    fi
    if [[ "$in_package" == true ]] && [[ "$line" =~ ^[[:space:]]*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
      echo "${BASH_REMATCH[1]}"
      return 0
    fi
  done < "$CARGO_TOML_PATH"

  echo "Could not find version in [package] section of $CARGO_TOML_PATH" >&2
  exit 1
}

# Change to repo root if in a git repository
if git rev-parse --show-toplevel &>/dev/null; then
  cd "$(git rev-parse --show-toplevel)"
fi

# Build release binaries if requested
if [[ "$BUILD_RELEASE" == true ]]; then
  echo "Building release binaries..."
  cargo build --release
fi

# Define files to include
BASE_FILES=(
  "target/release/bb_scrape${BINARY_EXT}"
  "target/release/cli${BINARY_EXT}"
  "README.md"
  "LICENSE"
)

FILES=("${BASE_FILES[@]}")
if [[ ${#EXTRA_FILES[@]} -gt 0 ]]; then
  FILES+=("${EXTRA_FILES[@]}")
fi

# Verify all files exist
for file in "${FILES[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "Error: File not found: $file" >&2
    echo "" >&2
    echo "The release binaries have not been built yet." >&2
    echo "Please run one of the following:" >&2
    echo "  1. Build and package: $0 --build-release" >&2
    echo "  2. Build first:       cargo build --release" >&2
    echo "     Then package:      $0" >&2
    exit 1
  fi
done

# Get version
VERSION=$(get_version)

# Create versioned output directory
VER_DIR="$OUT_DIR/v$VERSION"
mkdir -p "$VER_DIR"

ZIP_NAME="bb_scrape_v${VERSION}_${PLATFORM_ARCH}.zip"
ZIP_PATH="$VER_DIR/$ZIP_NAME"

# Remove existing zip if present
if [[ -f "$ZIP_PATH" ]]; then
  rm -f "$ZIP_PATH"
fi

# Create temporary directory for flattened structure
TEMP_DIR=$(mktemp -d -t bb_scrape_zip_XXXXXX)
trap 'rm -rf "$TEMP_DIR"' EXIT

# Copy files to temp directory with flattened structure
for file in "${FILES[@]}"; do
  filename=$(basename "$file")
  cp "$file" "$TEMP_DIR/$filename"
done

# Create zip archive
echo "Creating archive: $ZIP_PATH"
cd "$TEMP_DIR"
zip -q -9 -r "$ZIP_PATH" ./*
cd - > /dev/null

# Verify zip was created
if [[ ! -f "$ZIP_PATH" ]]; then
  echo "Expected zip not found at $ZIP_PATH (archive step failed)" >&2
  exit 1
fi

# Generate SHA256 checksum
HASH_FILE="$ZIP_PATH.sha256"
if command -v sha256sum &>/dev/null; then
  # Linux
  HASH=$(sha256sum "$ZIP_PATH" | awk '{print $1}')
elif command -v shasum &>/dev/null; then
  # macOS
  HASH=$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')
else
  echo "Warning: No SHA256 utility found, skipping checksum" >&2
  HASH=""
fi

if [[ -n "$HASH" ]]; then
  echo "$HASH  $ZIP_NAME" > "$HASH_FILE"
  echo "Created: $ZIP_PATH"
  echo "SHA256:  $HASH_FILE"
else
  echo "Created: $ZIP_PATH"
fi

echo "ZIP=$ZIP_PATH"
echo "SHA256=$HASH_FILE"
