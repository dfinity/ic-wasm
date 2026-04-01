# ic-wasm npm Package

This directory contains the npm package distribution system for `ic-wasm` using the **bundled binaries per-architecture** approach.

## Directory Structure

- **`ic-wasm/`** - Main wrapper package with binary wrapper script and programmatic API
- **`ic-wasm-{platform}-{arch}/`** - Platform-specific packages containing pre-compiled binaries
- **`scripts/`** - Build and deployment automation scripts
- **`Dockerfile.test`** - Docker-based testing environment
- **`docker-compose.test.yml`** - Multi-version Node.js testing


## Automated Release Process

The npm packages can be published via the GitHub Actions workflow:

1. Go to the repository's Actions tab
2. Select the "Publish to npm" workflow
3. Click "Run workflow" and provide:
   - **version**: Release version to download binaries from (e.g., `0.9.12`)
   - **npm_package_version** (optional): NPM package version if it should differ from the release version (e.g., `0.9.13`)
   - **beta**: Whether to publish as a beta release (tags packages with `beta` on npm)

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
./scripts/test-docker.sh full   # Full test on Node 18, 20, 22, 24
```

### 4. Update Version

```bash
./scripts/update-package-json.sh 0.9.11
```

This will update the version in all packages and their dependencies.

## Usage After Publishing

Users can install the package globally:

```bash
npm install -g @icp-sdk/ic-wasm
```

Or locally in their project:

```bash
npm install @icp-sdk/ic-wasm
```

Then use it:

```bash
ic-wasm --help
```

Or programmatically:

```javascript
const icWasm = require('@icp-sdk/ic-wasm');
console.log('Binary location:', icWasm.binaryPath);
```
