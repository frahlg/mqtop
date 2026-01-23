#!/bin/bash
set -e

# Update Homebrew formula with correct SHA256 checksums
# Usage: ./update-formula.sh v0.1.0

VERSION="${1:-$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.1.0")}"
VERSION_NUM="${VERSION#v}"
FORMULA="$(dirname "$0")/mqtop.rb"
REPO="frahlg/mqtop"

echo "Updating formula for version ${VERSION}..."

# Download and calculate checksums
calc_sha() {
    local asset=$1
    local url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
    echo "Fetching ${asset}..." >&2
    curl -fsSL "${url}" | shasum -a 256 | cut -d' ' -f1
}

# Get checksums for each platform
SHA_MACOS_ARM64=$(calc_sha "mqtop-macos-arm64")
SHA_MACOS_X64=$(calc_sha "mqtop-macos-x64")
SHA_LINUX_ARM64=$(calc_sha "mqtop-linux-arm64")
SHA_LINUX_X64=$(calc_sha "mqtop-linux-x64")

echo ""
echo "SHA256 checksums:"
echo "  macos-arm64: ${SHA_MACOS_ARM64}"
echo "  macos-x64:   ${SHA_MACOS_X64}"
echo "  linux-arm64: ${SHA_LINUX_ARM64}"
echo "  linux-x64:   ${SHA_LINUX_X64}"
echo ""

# Update formula
sed -i.bak \
    -e "s/version \".*\"/version \"${VERSION_NUM}\"/" \
    -e "s/PLACEHOLDER_SHA256_MACOS_ARM64/${SHA_MACOS_ARM64}/" \
    -e "s/PLACEHOLDER_SHA256_MACOS_X64/${SHA_MACOS_X64}/" \
    -e "s/PLACEHOLDER_SHA256_LINUX_ARM64/${SHA_LINUX_ARM64}/" \
    -e "s/PLACEHOLDER_SHA256_LINUX_X64/${SHA_LINUX_X64}/" \
    "${FORMULA}"

# Also update existing checksums (for subsequent updates)
sed -i.bak \
    -e "s/sha256 \"[a-f0-9]\{64\}\" # macos-arm64/sha256 \"${SHA_MACOS_ARM64}\" # macos-arm64/" \
    -e "s/sha256 \"[a-f0-9]\{64\}\" # macos-x64/sha256 \"${SHA_MACOS_X64}\" # macos-x64/" \
    -e "s/sha256 \"[a-f0-9]\{64\}\" # linux-arm64/sha256 \"${SHA_LINUX_ARM64}\" # linux-arm64/" \
    -e "s/sha256 \"[a-f0-9]\{64\}\" # linux-x64/sha256 \"${SHA_LINUX_X64}\" # linux-x64/" \
    "${FORMULA}"

rm -f "${FORMULA}.bak"

echo "Updated ${FORMULA}"
echo ""
echo "Next steps:"
echo "1. Copy formula to your homebrew-tap repo:"
echo "   cp ${FORMULA} ../homebrew-tap/Formula/"
echo "2. Commit and push the tap repo"
