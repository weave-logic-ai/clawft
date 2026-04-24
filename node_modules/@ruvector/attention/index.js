const { platform, arch } = process;

const platformPackages = {
  'linux-x64': '@ruvector/attention-linux-x64-gnu',
  'linux-arm64': '@ruvector/attention-linux-arm64-gnu',
  'darwin-x64': '@ruvector/attention-darwin-x64',
  'darwin-arm64': '@ruvector/attention-darwin-arm64',
  'win32-x64': '@ruvector/attention-win32-x64-msvc',
};

const key = `${platform}-${arch}`;
const pkg = platformPackages[key];

if (!pkg) {
  throw new Error(`Unsupported platform: ${key}. Supported: ${Object.keys(platformPackages).join(', ')}`);
}

module.exports = require(pkg);
