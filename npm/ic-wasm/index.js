// Main entry point for programmatic usage
const path = require('path');
const fs = require('fs');

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

function getBinaryPath() {
  if (!packageName) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }

  try {
    const platformPackage = require(packageName);
    if (platformPackage.binaryPath && fs.existsSync(platformPackage.binaryPath)) {
      return platformPackage.binaryPath;
    }
  } catch (err) {
    // Package not installed or not found
  }

  throw new Error(`Could not find ic-wasm binary for ${platform}-${arch}. Make sure the platform-specific package ${packageName} is installed.`);
}

// Cache the binary path once found
let cachedBinaryPath = null;

module.exports = {
  version: require('./package.json').version,

  // Lazy-load binary path using a getter
  get binaryPath() {
    if (cachedBinaryPath === null) {
      cachedBinaryPath = getBinaryPath();
    }
    return cachedBinaryPath;
  },

  // Also export the function for manual control
  getBinaryPath: getBinaryPath
};
