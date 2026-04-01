# ic-wasm

npm package for [ic-wasm](https://github.com/dfinity/ic-wasm) with pre-compiled binaries.

## Installation

```bash
npm install -g ic-wasm
```

Or add to your project:

```bash
npm install --save-dev ic-wasm
```

## Usage

```bash
ic-wasm --help
ic-wasm --version
```

### Common Commands

```bash
# Optimize a Wasm file
ic-wasm input.wasm -o output.wasm shrink

# Metadata operations
ic-wasm input.wasm -o output.wasm metadata candid:service -d candid.did -v public
```

## How it Works

This package uses platform-specific optional dependencies to install the correct pre-compiled binary for your system. The binary is ready to use immediately after installation - no additional downloads required.

### Supported Platforms

- macOS ARM64 (Apple Silicon)
- macOS x64 (Intel)
- Linux ARM64
- Linux x64
- Windows x64

### Programmatic Usage

```javascript
const icWasm = require('ic-wasm');

console.log('ic-wasm binary location:', icWasm.binaryPath);
console.log('ic-wasm version:', icWasm.version);
```

## Links

- [ic-wasm GitHub Repository](https://github.com/dfinity/ic-wasm)
- [Internet Computer Documentation](https://internetcomputer.org/docs/current/developer-docs/)
- [ic-wasm Releases](https://github.com/dfinity/ic-wasm/releases)
