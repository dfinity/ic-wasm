#!/bin/bash

# Script to verify that all binaries are present and executable

set -e

# Ensure script is run from npm directory
if [ ! -d "ic-wasm" ] || [ ! -f "ic-wasm/package.json" ]; then
  echo "Error: This script must be run from the npm directory"
  echo "Usage: cd npm && ./scripts/verify-binaries.sh"
  exit 1
fi

echo "Verifying binary files..."

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

MISSING=0
TOTAL=0

# Check each platform binary
check_binary() {
  local platform=$1
  local binary_name=$2
  local binary_path="ic-wasm-${platform}/bin/${binary_name}"
  
  if [ -f "$binary_path" ]; then
    # Check if executable (on Unix systems)
    if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" ]]; then
      if [ -x "$binary_path" ]; then
        echo -e "${GREEN}✓${NC} $binary_path (executable)"
      else
        echo -e "${YELLOW}⚠${NC} $binary_path (exists but not executable)"
        echo "  Run: chmod +x $binary_path"
      fi
    else
      echo -e "${GREEN}✓${NC} $binary_path"
    fi
    
    # Show file size
    if command -v du &> /dev/null; then
      SIZE=$(du -h "$binary_path" | cut -f1)
      echo "  Size: $SIZE"
    fi
  else
    echo -e "${RED}✗${NC} $binary_path (missing)"
    MISSING=$((MISSING + 1))
  fi
  echo ""
}

echo ""

# Dynamically find all platform packages
for platform_dir in ic-wasm-*/; do
  # Skip if no directories found
  [ -d "$platform_dir" ] || continue
  
  # Remove trailing slash and ic-wasm- prefix to get platform name
  platform="${platform_dir%/}"
  platform="${platform#ic-wasm-}"
  
  # Determine binary name based on platform
  if [[ "$platform" == win32-* ]]; then
    binary_name="ic-wasm.exe"
  else
    binary_name="ic-wasm"
  fi
  
  check_binary "$platform" "$binary_name"
  TOTAL=$((TOTAL + 1))
done

# Summary
echo "================================================"
if [ $MISSING -eq 0 ]; then
  echo -e "${GREEN}All binaries present!${NC} ($TOTAL/$TOTAL)"
  echo ""
  echo "Next steps:"
  echo "  1. Test installation: ./scripts/test-docker.sh"
  echo "  2. Publish packages: ./scripts/publish-all.sh <version>"
else
  echo -e "${RED}Missing binaries:${NC} $MISSING/$TOTAL"
  echo ""
  echo "Download missing binaries with:"
  echo "  ./scripts/download-binaries.sh 0.9.11"
  exit 1
fi
