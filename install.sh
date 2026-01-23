#!/bin/bash
set -e

# mqtop Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/frahlg/mqtop/master/install.sh | bash

REPO="frahlg/mqtop"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="mqtop"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}==>${NC} $1"; }
warn() { echo -e "${YELLOW}==>${NC} $1"; }
error() { echo -e "${RED}Error:${NC} $1"; exit 1; }

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Darwin) os="macos" ;;
        Linux)  os="linux" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) error "Unsupported operating system: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x64" ;;
        arm64|aarch64) arch="arm64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

# Get latest release version from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
        grep '"tag_name":' |
        sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install
install() {
    local platform version download_url tmp_dir

    platform=$(detect_platform)
    info "Detected platform: ${platform}"

    # Get version (use provided or fetch latest)
    if [ -n "${VERSION}" ]; then
        version="${VERSION}"
    else
        info "Fetching latest version..."
        version=$(get_latest_version)
    fi

    if [ -z "${version}" ]; then
        error "Could not determine version. Check https://github.com/${REPO}/releases"
    fi

    info "Installing mqtop ${version}..."

    # Construct download URL
    case "${platform}" in
        windows-x64)
            download_url="https://github.com/${REPO}/releases/download/${version}/mqtop-windows-x64.exe"
            BINARY_NAME="mqtop.exe"
            ;;
        *)
            download_url="https://github.com/${REPO}/releases/download/${version}/mqtop-${platform}"
            ;;
    esac

    # Create temp directory
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "${tmp_dir}"' EXIT

    # Download
    info "Downloading from ${download_url}..."
    if ! curl -fsSL "${download_url}" -o "${tmp_dir}/${BINARY_NAME}"; then
        error "Download failed. Check if release ${version} exists at https://github.com/${REPO}/releases"
    fi

    # Make executable
    chmod +x "${tmp_dir}/${BINARY_NAME}"

    # Handle macOS Gatekeeper
    if [ "$(uname -s)" = "Darwin" ]; then
        info "Removing macOS quarantine attribute..."
        xattr -cr "${tmp_dir}/${BINARY_NAME}" 2>/dev/null || true
    fi

    # Install
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
    if [ -w "${INSTALL_DIR}" ]; then
        mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Need sudo to install to ${INSTALL_DIR}"
        sudo mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    # Verify installation
    if command -v mqtop &> /dev/null; then
        info "Successfully installed mqtop ${version}!"
        echo ""
        mqtop --version 2>/dev/null || echo "mqtop installed to ${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
        warn "Make sure ${INSTALL_DIR} is in your PATH"
    fi

    echo ""
    info "Run 'mqtop --help' to get started"
}

# Run installer
install
