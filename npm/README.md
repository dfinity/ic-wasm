# ic-wasm npm Package

This directory contains the npm package distribution system for `ic-wasm` using the **bundled binaries per-architecture** approach.

## Directory Structure

- **`ic-wasm/`** - Main wrapper package with binary wrapper script and programmatic API
- **`ic-wasm-{platform}-{arch}/`** - Platform-specific packages containing pre-compiled binaries
- **`scripts/`** - Build and deployment automation scripts
- **`Dockerfile.test`** - Docker-based testing environment
- **`docker-compose.test.yml`** - Multi-version Node.js testing


## Automated Release Process

The npm packages are automatically published when a GitHub release is created:

1. Developer pushes a version tag (e.g., `v0.9.12`) to the main ic-wasm repository
2. The `release.yml` workflow (managed by cargo-dist) builds and releases Rust binaries
3. When the GitHub release is published, the `release-npm.yml` workflow automatically:
   - Downloads the newly released binaries from the GitHub release
   - Updates package.json versions to match
   - Runs Docker tests
   - Publishes all 6 packages to npm

See [`.github/workflows/release-npm.yml`](../.github/workflows/release-npm.yml) for the complete workflow.

## Manual Testing (for Development)

If you need to test the npm packages locally before a release:

### 1. Download Binaries

From the `npm` directory, download the pre-compiled binaries:

```bash
cd npm
./scripts/download-binaries.sh v0.9.11
```

Or manually download from [ic-wasm releases](https://github.com/dfinity/ic-wasm/releases) and place them in the respective `bin/` directories.

### 2. Verify Binaries

```bash
./scripts/verify-binaries.sh
```

### 3. Test Locally

```bash
./scripts/test-docker.sh quick  # Quick test on Node 20
./scripts/test-docker.sh full   # Full test on Node 18, 20, 22
```

### 4. Update Version

```bash
./scripts/update-package-json.sh 0.9.11
```

This will update the version in all packages and their dependencies.

## Usage After Publishing

Users can install the package globally:

```bash
npm install -g ic-wasm
```

Or locally in their project:

```bash
npm install ic-wasm
```

Then use it:

```bash
ic-wasm --help
```

Or programmatically:

```javascript
const icWasm = require('ic-wasm');
console.log('Binary location:', icWasm.binaryPath);
```
