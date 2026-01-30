#!/bin/bash

# Ensure script is run from npm directory
if [ ! -d "ic-wasm" ] || [ ! -f "ic-wasm/package.json" ]; then
  echo "Error: This script must be run from the npm directory"
  echo "Usage: cd npm && ./scripts/update-package-json.sh <new-version>"
  exit 1
fi

# Check if version argument is provided
if [ -z "$1" ]; then
  echo "Usage: ./scripts/update-package-json.sh <new-version>"
  echo "Example: ./scripts/update-package-json.sh 0.9.11"
  exit 1
fi

# Check if jq is installed
if ! command -v jq &> /dev/null; then
  echo "Error: jq is required but not installed"
  echo "Install with: brew install jq (macOS) or apt-get install jq (Linux)"
  exit 1
fi

NEW_VERSION="$1"

echo "Updating all packages to version $NEW_VERSION"

# Update platform packages
for dir in ic-wasm-*; do
  if [ -d "$dir" ]; then
    echo "Updating $dir..."
    cd "$dir"
    npm version "$NEW_VERSION" --no-git-tag-version
    cd ..
  fi
done

# Update main package
echo "Updating ic-wasm..."
cd ic-wasm
npm version "$NEW_VERSION" --no-git-tag-version
cd ..

# Update optionalDependencies in ic-wasm/package.json
echo "Updating optionalDependencies in ic-wasm/package.json..."
jq --arg version "$NEW_VERSION" \
  '.optionalDependencies = (.optionalDependencies | to_entries | map(.value = $version) | from_entries)' \
  ic-wasm/package.json > ic-wasm/package.json.tmp
mv ic-wasm/package.json.tmp ic-wasm/package.json
echo "✓ optionalDependencies updated to $NEW_VERSION"

echo ""
echo "All packages updated to $NEW_VERSION"
