#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * Find the platform-specific binary
 */
function findBinary() {
  const platform = process.platform;
  const arch = process.arch;

  // Map Node.js platform/arch to package names
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
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error(`Supported platforms: ${Object.keys(platformMap).join(', ')}`);
    process.exit(1);
  }

  // Try to find the binary in node_modules
  const binaryName = platform === 'win32' ? 'ic-wasm.exe' : 'ic-wasm';
  const possiblePaths = [
    // When installed globally (scoped packages are siblings)
    path.join(__dirname, '..', '..', '..', packageName, 'bin', binaryName),
    // When installed locally
    path.join(__dirname, '..', 'node_modules', packageName, 'bin', binaryName),
    // Alternative local path
    path.join(process.cwd(), 'node_modules', packageName, 'bin', binaryName)
  ];

  for (const binaryPath of possiblePaths) {
    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }
  }

  console.error(`Could not find ic-wasm binary for ${platform}-${arch}`);
  console.error(`Package ${packageName} may not have installed correctly.`);
  console.error(`Searched paths: ${possiblePaths.join(', ')}`);
  process.exit(1);
}

/**
 * Execute the binary with all arguments
 */
function run() {
  const binaryPath = findBinary();
  const args = process.argv.slice(2);

  const child = spawn(binaryPath, args, {
    stdio: 'inherit'
  });

  child.on('exit', (code) => {
    process.exit(code);
  });

  child.on('error', (err) => {
    console.error('Error executing ic-wasm:', err.message);
    process.exit(1);
  });
}

run();
