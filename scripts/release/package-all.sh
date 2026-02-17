#!/usr/bin/env bash
# Package all built targets for a release.
#
# Usage:
#   ./scripts/release/package-all.sh <version>
#
# Expects binaries at standard cross-compilation target paths under
# the project's target/ directory.  Only targets whose binaries are
# present will be packaged; missing targets are silently skipped.
#
# Output:
#   dist/weft-<version>-<target>.zip   (one per target)
#   dist/weft-<version>-<target>.sha256
#   dist/checksums-<version>.txt       (combined checksum manifest)

set -euo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
BINARY_NAME="weft"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PACKAGE_SCRIPT="${SCRIPT_DIR}/package.sh"

# Target triples and their human-readable labels.
declare -a TARGETS=(
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-msvc"
)

declare -A TARGET_LABELS=(
    ["x86_64-unknown-linux-musl"]="Linux x86_64"
    ["aarch64-unknown-linux-musl"]="Linux aarch64"
    ["x86_64-apple-darwin"]="macOS x86_64"
    ["aarch64-apple-darwin"]="macOS aarch64"
    ["x86_64-pc-windows-msvc"]="Windows x86_64"
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
usage() {
    echo "Usage: $0 <version>"
    echo ""
    echo "Arguments:"
    echo "  version   Release version (e.g. 0.1.0)"
    echo ""
    echo "Expects cross-compiled binaries at:"
    echo "  target/<triple>/release/${BINARY_NAME}[.exe]"
    exit 1
}

info() {
    echo "==> $*"
}

error() {
    echo "ERROR: $*" >&2
    exit 1
}

# Portable SHA-256: prefer sha256sum, fall back to shasum -a 256.
sha256() {
    if command -v sha256sum &>/dev/null; then
        sha256sum "$@"
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$@"
    else
        error "Neither sha256sum nor shasum found. Cannot compute checksums."
    fi
}

# Human-readable file size.
human_size() {
    local file="$1"
    if command -v numfmt &>/dev/null; then
        stat --printf='%s' "$file" 2>/dev/null | numfmt --to=iec-i --suffix=B
    elif stat -f '%z' "$file" &>/dev/null; then
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
if [[ $# -lt 1 ]]; then
    usage
fi

VERSION="$1"

if [[ -z "${VERSION}" ]]; then
    error "Version must not be empty."
fi

if [[ ! -x "${PACKAGE_SCRIPT}" ]]; then
    error "package.sh not found or not executable at '${PACKAGE_SCRIPT}'."
fi

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
DIST_DIR="${PROJECT_ROOT}/dist"
mkdir -p "${DIST_DIR}"

PACKAGED=0
SKIPPED=0
declare -a PACKAGED_TARGETS=()

echo ""
echo "Packaging ${BINARY_NAME} v${VERSION} for all available targets"
echo "============================================================="
echo ""

for target in "${TARGETS[@]}"; do
    label="${TARGET_LABELS[${target}]}"

    # Determine binary path (with .exe for Windows).
    if [[ "${target}" == *windows* ]]; then
        binary_path="${PROJECT_ROOT}/target/${target}/release/${BINARY_NAME}.exe"
    else
        binary_path="${PROJECT_ROOT}/target/${target}/release/${BINARY_NAME}"
    fi

    if [[ -f "${binary_path}" ]]; then
        info "${label} (${target}) -- found"
        "${PACKAGE_SCRIPT}" "${VERSION}" "${target}" "${binary_path}"
        PACKAGED=$((PACKAGED + 1))
        PACKAGED_TARGETS+=("${target}")
    else
        echo "    ${label} (${target}) -- skipped (binary not found)"
        SKIPPED=$((SKIPPED + 1))
    fi
done

# ---------------------------------------------------------------------------
# Combined checksum manifest
# ---------------------------------------------------------------------------
COMBINED_CHECKSUMS="${DIST_DIR}/checksums-${VERSION}.txt"
: > "${COMBINED_CHECKSUMS}"

for target in "${PACKAGED_TARGETS[@]}"; do
    zip_name="${BINARY_NAME}-${VERSION}-${target}.zip"
    zip_path="${DIST_DIR}/${zip_name}"
    if [[ -f "${zip_path}" ]]; then
        (cd "${DIST_DIR}" && sha256 "${zip_name}") >> "${COMBINED_CHECKSUMS}"
    fi
done

if [[ -s "${COMBINED_CHECKSUMS}" ]]; then
    info "Combined checksums written to ${COMBINED_CHECKSUMS}"
fi

# ---------------------------------------------------------------------------
# Summary table
# ---------------------------------------------------------------------------
echo ""
echo "============================================================="
echo "Release Summary  --  ${BINARY_NAME} v${VERSION}"
echo "============================================================="
printf "%-35s %-10s %s\n" "TARGET" "STATUS" "SIZE"
echo "-------------------------------------------------------------"

for target in "${TARGETS[@]}"; do
    zip_path="${DIST_DIR}/${BINARY_NAME}-${VERSION}-${target}.zip"
    if [[ -f "${zip_path}" ]]; then
        size="$(human_size "${zip_path}")"
        printf "%-35s %-10s %s\n" "${target}" "OK" "${size}"
    else
        printf "%-35s %-10s %s\n" "${target}" "SKIPPED" "-"
    fi
done

echo "-------------------------------------------------------------"
echo "  Packaged: ${PACKAGED}   Skipped: ${SKIPPED}"
echo ""

if [[ ${PACKAGED} -eq 0 ]]; then
    echo "WARNING: No binaries were found. Build targets first with:"
    echo "  cargo build --release --target <triple>"
    echo ""
    exit 1
fi

info "All packages are in ${DIST_DIR}/"
info "Done."
