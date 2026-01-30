#!/usr/bin/env node

/**
 * Post-install script to verify binary installation
 */

const fs = require('fs');
const path = require('path');

const platform = process.platform;
const arch = process.arch;

const platformMap = {
  'darwin-arm64': '@icp-sdk/ic-wasm-darwin-arm64',
  'darwin-x64': '@icp-sdk/ic-wasm-darwin-x64',
  'linux-arm64': '@icp-sdk/ic-wasm-linux-arm64',
  'linux-x64': '@icp-sdk/ic-wasm-linux-x64',
  'win32-x64': '@icp-sdk/ic-wasm-win32-x64'
};

const key = `${platform}-${arch}`;
const packageName = platformMap[key];

if (!packageName) {
  console.log(`
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║   WARNING: Unsupported platform: ${platform}-${arch.padEnd(17)}║
║                                                           ║
║   Supported platforms:                                    ║
║   - macOS ARM64 (Apple Silicon)                          ║
║   - macOS x64 (Intel)                                    ║
║   - Linux ARM64                                          ║
║   - Linux x64                                            ║
║   - Windows x64                                          ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
`);
  process.exit(0); // Don't fail the install
}

// Try to find and verify the binary
try {
  const platformPackage = require(packageName);
  const binaryPath = platformPackage.binaryPath;
  
  if (binaryPath && fs.existsSync(binaryPath)) {
    // Ensure binary has execute permissions (important for Docker/CI)
    try {
      fs.chmodSync(binaryPath, 0o755);
    } catch (err) {
      // Ignore permission errors - might not have rights to chmod
    }
    
    console.log(`
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║   ic-wasm installed successfully!                        ║
║                                                           ║
║   Platform: ${key.padEnd(44)}║
║                                                           ║
║   Usage:                                                  ║
║     $ ic-wasm --help                                      ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
`);
  } else {
    console.log(`
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║   WARNING: Binary not found                              ║
║                                                           ║
║   Platform: ${key.padEnd(44)}║
║   Package: ${packageName.padEnd(45)}║
║                                                           ║
║   The platform-specific package may not have installed   ║
║   correctly. Try reinstalling:                           ║
║     $ npm install --force                                ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
`);
  }
} catch (err) {
  console.log(`
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║   WARNING: Platform package not found                    ║
║                                                           ║
║   Platform: ${key.padEnd(44)}║
║   Package: ${packageName.padEnd(45)}║
║                                                           ║
║   The platform-specific package may not have installed   ║
║   correctly. Try reinstalling:                           ║
║     $ npm install --force                                ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
`);
}
