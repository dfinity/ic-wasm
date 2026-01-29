#!/bin/bash

set -e

# Ensure script is run from npm directory
if [ ! -d "ic-wasm" ] || [ ! -f "ic-wasm/package.json" ]; then
  echo "Error: This script must be run from the npm directory"
  echo "Usage: cd npm && ./scripts/download-binaries.sh <version>"
  exit 1
fi

# Check if version argument is provided
if [ -z "$1" ]; then
  echo "Error: Version argument is required"
  echo "Usage: ./scripts/download-binaries.sh <version>"
  echo "Example: ./scripts/download-binaries.sh 0.9.11"
  exit 1
fi

# Strip leading 'v' if present since ic-wasm releases use version format without 'v'
VERSION="${1#v}"

echo "Downloading ic-wasm binaries version $VERSION"
echo ""

# Create bin directories if they don't exist
mkdir -p ic-wasm-darwin-arm64/bin
mkdir -p ic-wasm-darwin-x64/bin
mkdir -p ic-wasm-linux-arm64/bin
mkdir -p ic-wasm-linux-x64/bin
mkdir -p ic-wasm-win32-x64/bin

# Base URL for downloads
BASE_URL="https://github.com/dfinity/ic-wasm/releases/download/${VERSION}"

# macOS ARM64 (Apple Silicon)
echo "Downloading macOS ARM64..."
curl -L "${BASE_URL}/ic-wasm-aarch64-apple-darwin.tar.xz" -o darwin-arm64.tar.xz
mkdir -p tmp-darwin-arm64
tar -xJf darwin-arm64.tar.xz -C tmp-darwin-arm64
find tmp-darwin-arm64 -name 'ic-wasm' -type f -exec mv {} ic-wasm-darwin-arm64/bin/ic-wasm \;
chmod +x ic-wasm-darwin-arm64/bin/ic-wasm
rm -rf darwin-arm64.tar.xz tmp-darwin-arm64
echo "✓ macOS ARM64 downloaded"
echo ""

# macOS x64 (Intel)
echo "Downloading macOS x64..."
curl -L "${BASE_URL}/ic-wasm-x86_64-apple-darwin.tar.xz" -o darwin-x64.tar.xz
mkdir -p tmp-darwin-x64
tar -xJf darwin-x64.tar.xz -C tmp-darwin-x64
find tmp-darwin-x64 -name 'ic-wasm' -type f -exec mv {} ic-wasm-darwin-x64/bin/ic-wasm \;
chmod +x ic-wasm-darwin-x64/bin/ic-wasm
rm -rf darwin-x64.tar.xz tmp-darwin-x64
echo "✓ macOS x64 downloaded"
echo ""

# Linux ARM64
echo "Downloading Linux ARM64..."
curl -L "${BASE_URL}/ic-wasm-aarch64-unknown-linux-gnu.tar.xz" -o linux-arm64.tar.xz
mkdir -p tmp-linux-arm64
tar -xJf linux-arm64.tar.xz -C tmp-linux-arm64
find tmp-linux-arm64 -name 'ic-wasm' -type f -exec mv {} ic-wasm-linux-arm64/bin/ic-wasm \;
chmod +x ic-wasm-linux-arm64/bin/ic-wasm
rm -rf linux-arm64.tar.xz tmp-linux-arm64
echo "✓ Linux ARM64 downloaded"
echo ""

# Linux x64
echo "Downloading Linux x64..."
curl -L "${BASE_URL}/ic-wasm-x86_64-unknown-linux-gnu.tar.xz" -o linux-x64.tar.xz
mkdir -p tmp-linux-x64
tar -xJf linux-x64.tar.xz -C tmp-linux-x64
find tmp-linux-x64 -name 'ic-wasm' -type f -exec mv {} ic-wasm-linux-x64/bin/ic-wasm \;
chmod +x ic-wasm-linux-x64/bin/ic-wasm
rm -rf linux-x64.tar.xz tmp-linux-x64
echo "✓ Linux x64 downloaded"
echo ""

# Windows x64
echo "Downloading Windows x64..."
curl -L "${BASE_URL}/ic-wasm-x86_64-pc-windows-msvc.zip" -o win32-x64.zip
mkdir -p tmp-win32-x64
unzip -o win32-x64.zip -d tmp-win32-x64
find tmp-win32-x64 -name 'ic-wasm.exe' -type f -exec mv {} ic-wasm-win32-x64/bin/ic-wasm.exe \;
rm -rf win32-x64.zip tmp-win32-x64
echo "✓ Windows x64 downloaded"

echo ""
echo "=========================================="
echo "All binaries downloaded successfully!"
echo "=========================================="
echo ""
echo "Binary locations:"
echo "  • ic-wasm-darwin-arm64/bin/ic-wasm"
echo "  • ic-wasm-darwin-x64/bin/ic-wasm"
echo "  • ic-wasm-linux-arm64/bin/ic-wasm"
echo "  • ic-wasm-linux-x64/bin/ic-wasm"
echo "  • ic-wasm-win32-x64/bin/ic-wasm.exe"
echo ""
