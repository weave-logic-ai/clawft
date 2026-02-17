# Release Process

clawft uses semantic versioning and GitHub Releases for distribution. Tagging a
version in git triggers CI to build platform binaries, a WASM module, and a
Docker image.

## Version Numbering

Versions follow [Semantic Versioning](https://semver.org/):

- **MAJOR.MINOR.PATCH** (e.g., `0.3.1`)
- Bump **MAJOR** for breaking API/config changes
- Bump **MINOR** for new features (backwards-compatible)
- Bump **PATCH** for bug fixes

Pre-release versions use a suffix: `0.3.0-alpha.1`, `0.3.0-rc.1`.

## Release Workflow

1. **Update version** in workspace `Cargo.toml` and crate `Cargo.toml` files.
2. **Create a git tag**:

   ```bash
   git tag -a v0.3.1 -m "Release v0.3.1"
   git push origin v0.3.1
   ```

3. **CI builds artifacts**: The release workflow builds binaries for all
   supported targets, creates a GitHub Release, and pushes the Docker image.

## Release Artifacts

Each release publishes the following artifacts:

| Artifact                     | Target                           | Notes                     |
|------------------------------|----------------------------------|---------------------------|
| `weft-linux-x86_64`         | `x86_64-unknown-linux-musl`      | Static binary, ~5 MB      |
| `weft-linux-aarch64`        | `aarch64-unknown-linux-musl`     | ARM64 servers, Raspberry Pi|
| `weft-macos-x86_64`         | `x86_64-apple-darwin`            | Intel Macs                |
| `weft-macos-aarch64`        | `aarch64-apple-darwin`           | Apple Silicon             |
| `weft-windows-x86_64.exe`   | `x86_64-pc-windows-msvc`        | Windows                   |
| `clawft_wasm.wasm`          | `wasm32-wasip2`                  | WASM module, < 300 KB     |
| Docker image                | `ghcr.io/clawft/clawft:vX.Y.Z`  | FROM scratch, ~5 MB       |

## Downloading a Release

### From GitHub

Download binaries from the
[Releases page](https://github.com/clawft/clawft/releases):

```bash
# Example: download the latest Linux x86_64 binary
curl -L -o weft \
  https://github.com/clawft/clawft/releases/latest/download/weft-linux-x86_64
chmod +x weft
```

### Verifying Checksums

Each release includes a `checksums.txt` file with SHA-256 hashes. Verify your
download:

```bash
curl -L -o checksums.txt \
  https://github.com/clawft/clawft/releases/latest/download/checksums.txt

sha256sum -c checksums.txt --ignore-missing
```

## Installing from a Release

Move the binary to a directory on your `PATH`:

```bash
# Linux / macOS
sudo mv weft /usr/local/bin/
weft --version

# Or install to user-local bin
mv weft ~/.local/bin/
```

On Windows, move `weft-windows-x86_64.exe` to a directory in your `PATH` and
rename it to `weft.exe`.

## Docker Image

Pull a specific release:

```bash
docker pull ghcr.io/clawft/clawft:v0.3.1
```

Or use the `latest` tag:

```bash
docker pull ghcr.io/clawft/clawft:latest
```

See the [Docker Deployment Guide](docker.md) for full usage instructions.

## Building from Source

### Prerequisites

- **Rust 1.93+** (edition 2024):

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  ```

- **Git**

### Build Steps

```bash
git clone https://github.com/clawft/clawft.git
cd clawft
cargo build --release
```

The binary is at `target/release/weft`.

### Cross-Compilation

To build for a different target (e.g., ARM64 Linux):

```bash
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

### WASM Build

```bash
rustup target add wasm32-wasip2
cargo build -p clawft-wasm --target wasm32-wasip2 --release
```

See the [WASM Deployment Guide](wasm.md) for runtime instructions.
