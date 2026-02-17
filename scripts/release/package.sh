#!/usr/bin/env bash
# Package release binaries with documentation for distribution.
#
# Usage:
#   ./scripts/release/package.sh <version> <target> <binary-path>
#
# Examples:
#   ./scripts/release/package.sh 0.1.0 x86_64-unknown-linux-musl target/x86_64-unknown-linux-musl/release/weft
#   ./scripts/release/package.sh 0.1.0 aarch64-apple-darwin target/aarch64-apple-darwin/release/weft
#
# Output:
#   dist/weft-<version>-<target>.zip
#   dist/weft-<version>-<target>.sha256

set -euo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
BINARY_NAME="weft"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
usage() {
    echo "Usage: $0 <version> <target> <binary-path>"
    echo ""
    echo "Arguments:"
    echo "  version       Release version (e.g. 0.1.0)"
    echo "  target        Target triple (e.g. x86_64-unknown-linux-musl)"
    echo "  binary-path   Path to the compiled binary"
    echo ""
    echo "Output:"
    echo "  dist/${BINARY_NAME}-<version>-<target>.zip"
    echo "  dist/${BINARY_NAME}-<version>-<target>.sha256"
    exit 1
}

info() {
    echo "==> $*"
}

error() {
    echo "ERROR: $*" >&2
    exit 1
}

# Portable SHA-256 checksum: prefer sha256sum, fall back to shasum -a 256.
sha256() {
    if command -v sha256sum &>/dev/null; then
        sha256sum "$@"
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$@"
    else
        error "Neither sha256sum nor shasum found. Cannot compute checksums."
    fi
}

# Return just the hash from a sha256 invocation.
sha256_hash() {
    sha256 "$@" | awk '{print $1}'
}

# Human-readable file size.
human_size() {
    local file="$1"
    if command -v numfmt &>/dev/null; then
        stat --printf='%s' "$file" 2>/dev/null | numfmt --to=iec-i --suffix=B
    elif stat -f '%z' "$file" &>/dev/null; then
        # macOS stat
        local bytes
        bytes=$(stat -f '%z' "$file")
        echo "$((bytes / 1024))KiB"
    else
        wc -c < "$file" | tr -d ' '
    fi
}

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------
if [[ $# -lt 3 ]]; then
    usage
fi

VERSION="$1"
TARGET="$2"
BINARY_PATH="$3"

if [[ -z "${VERSION}" ]]; then
    error "Version must not be empty."
fi
if [[ -z "${TARGET}" ]]; then
    error "Target must not be empty."
fi
if [[ ! -f "${BINARY_PATH}" ]]; then
    error "Binary not found at '${BINARY_PATH}'."
fi

# ---------------------------------------------------------------------------
# Derived paths
# ---------------------------------------------------------------------------
PACKAGE_NAME="${BINARY_NAME}-${VERSION}-${TARGET}"
DIST_DIR="${PROJECT_ROOT}/dist"
STAGING_DIR="${DIST_DIR}/staging/${PACKAGE_NAME}"
ZIP_FILE="${DIST_DIR}/${PACKAGE_NAME}.zip"
SHA_FILE="${DIST_DIR}/${PACKAGE_NAME}.sha256"

# ---------------------------------------------------------------------------
# Detect Windows target (binary will have .exe suffix)
# ---------------------------------------------------------------------------
is_windows_target() {
    [[ "${TARGET}" == *windows* || "${TARGET}" == *pc-windows* ]]
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
info "Packaging ${PACKAGE_NAME}"

# Clean up any prior staging for this package.
rm -rf "${STAGING_DIR}"
mkdir -p "${STAGING_DIR}"
mkdir -p "${DIST_DIR}"

# 1. Copy binary -----------------------------------------------------------
if is_windows_target; then
    DEST_BINARY="${STAGING_DIR}/${BINARY_NAME}.exe"
else
    DEST_BINARY="${STAGING_DIR}/${BINARY_NAME}"
fi

cp "${BINARY_PATH}" "${DEST_BINARY}"
chmod +x "${DEST_BINARY}"
info "  Binary:  $(basename "${DEST_BINARY}")"

# 2. Copy documentation (skip gracefully if missing) -----------------------
copy_if_exists() {
    local src="$1"
    local dest_name="${2:-$(basename "$1")}"
    if [[ -f "${src}" ]]; then
        cp "${src}" "${STAGING_DIR}/${dest_name}"
        info "  Doc:     ${dest_name}"
    fi
}

copy_if_exists "${PROJECT_ROOT}/README.md"
copy_if_exists "${PROJECT_ROOT}/LICENSE"
copy_if_exists "${PROJECT_ROOT}/CHANGELOG.md"
copy_if_exists "${PROJECT_ROOT}/docs/getting-started/quickstart.md" "quickstart.md"
copy_if_exists "${PROJECT_ROOT}/docs/guides/configuration.md" "configuration.md"

# 3. Generate checksums.txt inside the package ------------------------------
BINARY_BASENAME="$(basename "${DEST_BINARY}")"
(cd "${STAGING_DIR}" && sha256 "${BINARY_BASENAME}" > checksums.txt)
info "  Checksum: checksums.txt"

# 4. Create archive ---------------------------------------------------------
rm -f "${ZIP_FILE}"
if command -v zip &>/dev/null; then
    (cd "${DIST_DIR}/staging" && zip -r "${ZIP_FILE}" "${PACKAGE_NAME}")
else
    # Fall back to tar+gzip when zip is unavailable.
    ZIP_FILE="${DIST_DIR}/${PACKAGE_NAME}.tar.gz"
    SHA_FILE="${DIST_DIR}/${PACKAGE_NAME}.tar.gz.sha256"
    (cd "${DIST_DIR}/staging" && tar czf "${ZIP_FILE}" "${PACKAGE_NAME}")
fi
info "  Archive: ${ZIP_FILE}"

# 5. Create standalone checksum file ----------------------------------------
(cd "${DIST_DIR}" && sha256 "$(basename "${ZIP_FILE}")" > "${SHA_FILE}")
info "  SHA256:  ${SHA_FILE}"

# 6. Print summary ----------------------------------------------------------
echo ""
echo "--- Package Summary ---"
echo "  Name:     $(basename "${ZIP_FILE}")"
echo "  Size:     $(human_size "${ZIP_FILE}")"
echo "  SHA256:   $(sha256_hash "${ZIP_FILE}")"
echo "  Contents:"
(cd "${DIST_DIR}/staging" && find "${PACKAGE_NAME}" -type f | sort | while read -r f; do
    echo "    ${f}"
done)
echo ""

# 7. Cleanup staging --------------------------------------------------------
rm -rf "${STAGING_DIR}"
info "Staging directory cleaned up."
info "Done."
