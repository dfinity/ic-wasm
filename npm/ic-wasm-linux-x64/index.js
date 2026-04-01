const path = require('path');

module.exports = {
  binaryPath: path.join(__dirname, 'bin', process.platform === 'win32' ? 'ic-wasm.exe' : 'ic-wasm')
};
