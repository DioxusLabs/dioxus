#!/bin/bash
# Install script for fixed dx binary
# Usage: curl -sSL https://raw.githubusercontent.com/akesson/dioxus/fix/custom-dx-build/install-dx.sh | bash
# Or: ./install-dx.sh [VERSION]

set -euo pipefail

# Configuration
REPO="akesson/dioxus"
DEFAULT_VERSION="v0.6.3-fix.1"
VERSION=${1:-$DEFAULT_VERSION}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Detect platform and architecture
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Darwin*)  os="apple-darwin" ;;
        Linux*)   os="unknown-linux-gnu" ;;
        CYGWIN*)  os="pc-windows-msvc" ;;
        MINGW*)   os="pc-windows-msvc" ;;
        *)        error "Unsupported operating system: $(uname -s)" ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)        error "Unsupported architecture: $(uname -m)" ;;
    esac
    
    echo "${arch}-${os}"
}

# Main installation function
main() {
    info "Installing fixed dx binary version $VERSION"
    
    local target
    target=$(detect_platform)
    info "Detected platform: $target"
    
    local url="https://github.com/${REPO}/releases/download/${VERSION}/dx-${target}-${VERSION}.tar.gz"
    local install_dir="${HOME}/.local/bin"
    
    # Create install directory if it doesn't exist
    mkdir -p "$install_dir"
    
    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        info "Adding $install_dir to PATH. Add this to your shell profile:"
        info "export PATH=\"$install_dir:\$PATH\""
        export PATH="$install_dir:$PATH"
    fi
    
    # Download and install
    info "Downloading from: $url"
    
    local temp_dir
    temp_dir=$(mktemp -d)
    
    # Download and extract
    if command -v curl &> /dev/null; then
        curl -L "$url" | tar -xz -C "$temp_dir"
    elif command -v wget &> /dev/null; then
        wget -O- "$url" | tar -xz -C "$temp_dir"
    else
        error "Neither curl nor wget is available"
    fi
    
    # Find the binary (handle different extraction layouts)
    local binary_path
    if [[ -f "$temp_dir/dx" ]]; then
        binary_path="$temp_dir/dx"
    elif [[ -f "$temp_dir/dx.exe" ]]; then
        binary_path="$temp_dir/dx.exe"
    else
        error "Could not find dx binary in downloaded archive"
    fi
    
    # Install the binary
    cp "$binary_path" "$install_dir/dx"
    chmod +x "$install_dir/dx"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    info "Successfully installed dx to $install_dir/dx"
    
    # Verify installation
    if "$install_dir/dx" --version &> /dev/null; then
        info "Installation verified: $("$install_dir/dx" --version)"
        info ""
        info "🎉 Fixed dx binary installed successfully!"
        info ""
        info "Usage examples:"
        info "  dx --version"
        info "  dx build --platform web --release"
        info "  dx serve --platform web"
    else
        warn "Installation completed but dx binary verification failed"
        warn "You may need to add $install_dir to your PATH"
    fi
}

# Run main function
main "$@"
