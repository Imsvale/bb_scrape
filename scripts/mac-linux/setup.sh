#!/usr/bin/env bash
# scripts/mac-linux/setup.sh
# Automated setup script for building bb_scrape on macOS/Linux
# For users with no prior development tools installed

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Detect OS
OS_NAME=$(uname -s)
case "$OS_NAME" in
  Linux*)
    PLATFORM="Linux"
    ;;
  Darwin*)
    PLATFORM="macOS"
    ;;
  *)
    echo -e "${RED}Error: Unsupported operating system: $OS_NAME${NC}"
    echo "This script only supports macOS and Linux."
    exit 1
    ;;
esac

echo "════════════════════════════════════════════════════════════"
echo "  bb_scrape - Automated Build Setup for $PLATFORM"
echo "════════════════════════════════════════════════════════════"
echo ""

# Function to print section headers
print_section() {
  echo ""
  echo -e "${BLUE}▶ $1${NC}"
  echo "────────────────────────────────────────────────────────────"
}

# Function to print success
print_success() {
  echo -e "${GREEN}✓ $1${NC}"
}

# Function to print warning
print_warning() {
  echo -e "${YELLOW}⚠ $1${NC}"
}

# Function to print error
print_error() {
  echo -e "${RED}✗ $1${NC}"
}

# Function to ask yes/no question
ask_yes_no() {
  while true; do
    read -p "$1 (y/n): " yn
    case $yn in
      [Yy]* ) return 0;;
      [Nn]* ) return 1;;
      * ) echo "Please answer yes (y) or no (n).";;
    esac
  done
}

# Step 1: Check for development tools
print_section "Step 1: Checking for development tools"

if [[ "$PLATFORM" == "macOS" ]]; then
  # Check for Xcode Command Line Tools
  if xcode-select -p &>/dev/null; then
    print_success "Xcode Command Line Tools already installed"
  else
    print_warning "Xcode Command Line Tools not found"
    echo ""
    echo "These tools provide the C compiler and other essentials needed by Rust."
    echo ""
    if ask_yes_no "Install Xcode Command Line Tools now?"; then
      echo ""
      echo "A popup will appear. Click 'Install' and wait for completion..."
      xcode-select --install
      echo ""
      echo "Press Enter when installation is complete..."
      read

      # Verify installation
      if xcode-select -p &>/dev/null; then
        print_success "Xcode Command Line Tools installed successfully"
      else
        print_error "Installation failed or was cancelled"
        exit 1
      fi
    else
      print_error "Cannot proceed without Xcode Command Line Tools"
      exit 1
    fi
  fi
elif [[ "$PLATFORM" == "Linux" ]]; then
  # Check for essential build tools
  MISSING_TOOLS=()

  if ! command -v gcc &>/dev/null && ! command -v clang &>/dev/null; then
    MISSING_TOOLS+=("gcc or clang")
  fi

  if ! command -v make &>/dev/null; then
    MISSING_TOOLS+=("make")
  fi

  if ! command -v pkg-config &>/dev/null; then
    MISSING_TOOLS+=("pkg-config")
  fi

  if [[ ${#MISSING_TOOLS[@]} -gt 0 ]]; then
    print_warning "Missing development tools: ${MISSING_TOOLS[*]}"
    echo ""
    echo "You need to install build essentials. Run one of these commands:"
    echo ""
    echo "  Ubuntu/Debian:"
    echo "    sudo apt-get update"
    echo "    sudo apt-get install build-essential pkg-config libssl-dev"
    echo ""
    echo "  Fedora/RHEL:"
    echo "    sudo dnf groupinstall 'Development Tools'"
    echo "    sudo dnf install pkg-config openssl-devel"
    echo ""
    echo "  Arch Linux:"
    echo "    sudo pacman -S base-devel"
    echo ""
    if ask_yes_no "Have you installed these tools? Continue anyway?"; then
      print_warning "Proceeding... build may fail if tools are missing"
    else
      exit 1
    fi
  else
    print_success "Development tools found"
  fi
fi

# Check for zip utility
if ! command -v zip &>/dev/null; then
  print_warning "zip utility not found (optional, only needed for packaging)"
else
  print_success "zip utility found"
fi

# Step 2: Check for Rust
print_section "Step 2: Checking for Rust toolchain"

if command -v cargo &>/dev/null && command -v rustc &>/dev/null; then
  RUST_VERSION=$(rustc --version | cut -d' ' -f2)
  print_success "Rust already installed (version $RUST_VERSION)"
else
  print_warning "Rust not found"
  echo ""
  echo "Rust is the programming language this project is written in."
  echo "The installer (rustup) will download and install:"
  echo "  - rustc (compiler)"
  echo "  - cargo (build tool & package manager)"
  echo "  - standard library"
  echo ""
  if ask_yes_no "Install Rust now?"; then
    echo ""
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Source cargo environment
    if [[ -f "$HOME/.cargo/env" ]]; then
      source "$HOME/.cargo/env"
    fi

    # Verify installation
    if command -v cargo &>/dev/null; then
      RUST_VERSION=$(rustc --version | cut -d' ' -f2)
      print_success "Rust installed successfully (version $RUST_VERSION)"
    else
      print_error "Rust installation failed"
      echo ""
      echo "Please restart your terminal and run this script again."
      exit 1
    fi
  else
    print_error "Cannot proceed without Rust"
    exit 1
  fi
fi

# Step 3: Check if we're in the project directory
print_section "Step 3: Verifying project location"

cd "$PROJECT_ROOT"

if [[ ! -f "Cargo.toml" ]]; then
  print_error "Cargo.toml not found in $PROJECT_ROOT"
  echo "This script must be run from the bb_scrape project directory."
  exit 1
fi

print_success "Found project at: $PROJECT_ROOT"

# Step 4: Build the project
print_section "Step 4: Building the project"

echo ""
echo "This will compile the project in release mode (optimized)."
echo "First build may take 10-20 minutes as it downloads and compiles dependencies."
echo ""

if ask_yes_no "Start building now?"; then
  echo ""
  echo "Building... (this may take a while)"
  echo ""

  if cargo build --release; then
    echo ""
    print_success "Build completed successfully!"
    echo ""
    echo "Binaries created:"
    echo "  GUI: $PROJECT_ROOT/target/release/bb_scrape"
    echo "  CLI: $PROJECT_ROOT/target/release/cli"
  else
    echo ""
    print_error "Build failed"
    exit 1
  fi
else
  print_warning "Build skipped"
  echo ""
  echo "You can build manually later with:"
  echo "  cd $PROJECT_ROOT"
  echo "  cargo build --release"
  exit 0
fi

# Step 5: Create package (optional)
print_section "Step 5: Create distribution package (optional)"

echo ""
echo "You can create a .zip package with flattened structure for easy distribution."
echo ""

if ask_yes_no "Create distribution package?"; then
  if [[ -f "$SCRIPT_DIR/build.sh" ]]; then
    echo ""
    "$SCRIPT_DIR/build.sh"
    echo ""
    print_success "Package created in artifacts/ directory"
  else
    print_error "Packaging script not found at $SCRIPT_DIR/build.sh"
  fi
else
  print_warning "Packaging skipped"
fi

# Final summary
echo ""
echo "════════════════════════════════════════════════════════════"
echo -e "${GREEN}  Setup Complete!${NC}"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "To run the GUI application:"
echo "  ./target/release/bb_scrape"
echo ""
echo "To run the CLI application:"
echo "  ./target/release/cli --help"
echo ""
echo "To rebuild after making changes:"
echo "  cargo build --release"
echo ""
echo "For more information, see README.md"
echo ""
