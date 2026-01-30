#!/bin/bash

set -e

# Ensure script is run from npm directory
if [ ! -d "ic-wasm" ] || [ ! -f "ic-wasm/package.json" ]; then
  echo "Error: This script must be run from the npm directory"
  echo "Usage: cd npm && ./scripts/publish-all.sh <version> [tag]"
  exit 1
fi

# Check if version argument is provided
if [ -z "$1" ]; then
  echo "Error: Version argument is required"
  echo "Usage: ./scripts/publish-all.sh <version> [tag]"
  echo "Example: ./scripts/publish-all.sh 0.9.11"
  echo "Example: ./scripts/publish-all.sh 0.9.11-beta beta"
  exit 1
fi

VERSION="$1"
NPM_TAG="${2:-}"

echo "Publishing version $VERSION"

# Array of platform packages
PLATFORMS=(
  "ic-wasm-darwin-arm64"
  "ic-wasm-darwin-x64"
  "ic-wasm-linux-arm64"
  "ic-wasm-linux-x64"
  "ic-wasm-win32-x64"
)

# Function to check package version
check_version() {
  local package_dir="$1"
  local package_json="$package_dir/package.json"
  
  if [ ! -f "$package_json" ]; then
    echo "Error: $package_json not found"
    exit 1
  fi
  
  local pkg_version=$(node -p "require('./$package_json').version")
  
  if [ "$pkg_version" != "$VERSION" ]; then
    echo "Error: Version mismatch in $package_dir"
    echo "  Expected: $VERSION"
    echo "  Found: $pkg_version"
    exit 1
  fi
  
  echo "✓ $package_dir version matches: $VERSION"
}

# Verify versions before publishing
echo "Verifying package versions..."
for platform in "${PLATFORMS[@]}"; do
  check_version "$platform"
done
check_version "ic-wasm"
echo "All versions verified!"
echo ""

# Determine npm dist-tag
BETA_TAG=""
if [ -n "$NPM_TAG" ]; then
  BETA_TAG="--tag $NPM_TAG"
  echo "Publishing with tag: $NPM_TAG"
else
  echo "Publishing as latest"
fi
echo ""

# Publish platform packages
for platform in "${PLATFORMS[@]}"; do
  echo "Publishing $platform..."
  cd "$platform"
  npm publish --access public --provenance $BETA_TAG
  cd ..
  echo "✓ $platform published"
done

# Publish main package
echo "Publishing main package ic-wasm..."
cd ic-wasm
npm publish --access public --provenance $BETA_TAG
cd ..
echo "✓ ic-wasm published"

echo ""
echo "All packages published successfully!"
