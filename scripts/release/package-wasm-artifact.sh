#!/usr/bin/env bash
# package-wasm-artifact.sh — version + checksum packaging for WASM release artefacts.
#
# WEFT-405: browser (and WASI) release artefacts get the same supply-chain
# treatment as cargo-dist natives: a VERSION.json, SHA-256 checksums, and
# (in CI) a GitHub Attestations / sigstore provenance attachment.
#
# Usage:
#   # Write VERSION.json into a package directory (browser-pkg/, etc.)
#   scripts/release/package-wasm-artifact.sh write-version \
#       --kind browser|wasi \
#       --target wasm32-unknown-unknown|wasm32-wasip2 \
#       --out-dir browser-pkg \
#       [--tag v0.6.20] [--git-sha abc123]
#
#   # Produce SHA-256 checksum files next to each artefact + a combined SUMS
#   scripts/release/package-wasm-artifact.sh checksum \
#       --out-dir dist \
#       clawft-browser-wasm-v0.6.20.tar.gz \
#       clawft-browser-wasm-v0.6.20.VERSION.json
#
#   # Browser-only: tar a pkg dir (must already contain VERSION.json) + checksum
#   scripts/release/package-wasm-artifact.sh package-browser \
#       --pkg-dir browser-pkg \
#       --tag v0.6.20 \
#       --out-dir .
#
#   # WASI-only: copy/rename wasm + VERSION.json + checksum
#   scripts/release/package-wasm-artifact.sh package-wasi \
#       --wasm path/to/clawft_wasm.wasm \
#       --tag v0.6.20 \
#       --out-dir .
#
# Outputs (printed as KEY=value lines for CI step consumption):
#   version_json=...
#   tarball=...          (package-browser)
#   asset=...            (package-wasi)
#   sha256sums=...
#   workspace_version=...
#
# Detached signature / attestation is performed by the calling workflow via
# actions/attest-build-provenance (same root of trust as release.yml). This
# script never holds signing keys.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

die() { echo "error: $*" >&2; exit 1; }
info() { echo "→ $*" >&2; }

# ---------------------------------------------------------------------------
# Resolve workspace version from root Cargo.toml
# ---------------------------------------------------------------------------
workspace_version() {
    # Prefer [workspace.package] version = "..."
    local v
    v=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
    if [[ -z "$v" ]]; then
        die "could not read workspace version from Cargo.toml"
    fi
    echo "$v"
}

git_sha_short() {
    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git rev-parse --short=12 HEAD 2>/dev/null || echo "unknown"
    else
        echo "unknown"
    fi
}

git_sha_full() {
    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git rev-parse HEAD 2>/dev/null || echo "unknown"
    else
        echo "unknown"
    fi
}

# sha256 of a file → "HEX  filename" (GNU sha256sum compatible, portable)
sha256_line() {
    local path="$1"
    local base
    base=$(basename "$path")
    if command -v sha256sum >/dev/null 2>&1; then
        # Prefer GNU form but force basename-only second field for portability
        local hex
        hex=$(sha256sum "$path" | awk '{print $1}')
        echo "${hex}  ${base}"
    elif command -v shasum >/dev/null 2>&1; then
        local hex
        hex=$(shasum -a 256 "$path" | awk '{print $1}')
        echo "${hex}  ${base}"
    else
        die "neither sha256sum nor shasum found"
    fi
}

iso_now() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# ---------------------------------------------------------------------------
# write-version
# ---------------------------------------------------------------------------
cmd_write_version() {
    local kind="" target="" out_dir="" tag="" git_sha="" extra_name=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --kind) kind="$2"; shift 2 ;;
            --target) target="$2"; shift 2 ;;
            --out-dir) out_dir="$2"; shift 2 ;;
            --tag) tag="$2"; shift 2 ;;
            --git-sha) git_sha="$2"; shift 2 ;;
            --name) extra_name="$2"; shift 2 ;;
            *) die "write-version: unknown arg: $1" ;;
        esac
    done
    [[ -n "$kind" ]] || die "write-version: --kind required"
    [[ -n "$target" ]] || die "write-version: --target required"
    [[ -n "$out_dir" ]] || die "write-version: --out-dir required"
    case "$kind" in browser|wasi) ;; *) die "write-version: --kind must be browser|wasi" ;; esac

    mkdir -p "$out_dir"
    local ws_ver
    ws_ver=$(workspace_version)
    local sha_full sha_short
    sha_full=${git_sha:-$(git_sha_full)}
    sha_short=$(echo "$sha_full" | cut -c1-12)
    local built
    built=$(iso_now)
    local tag_val="${tag:-unreleased}"

    local out="${out_dir}/VERSION.json"
    # Pure bash JSON (no jq dependency) — fields are controlled inputs.
    cat >"$out" <<EOF
{
  "schema": "weftos.wasm-artifact.v1",
  "kind": "${kind}",
  "target": "${target}",
  "package": "clawft-wasm",
  "workspace_version": "${ws_ver}",
  "tag": "${tag_val}",
  "git_sha": "${sha_full}",
  "git_sha_short": "${sha_short}",
  "built_at": "${built}",
  "name": "${extra_name:-clawft-wasm-${kind}}"
}
EOF
    info "wrote ${out}"
    echo "version_json=${out}"
    echo "workspace_version=${ws_ver}"
}

# ---------------------------------------------------------------------------
# checksum — generate per-file .sha256 + combined SHA256SUMS
# ---------------------------------------------------------------------------
cmd_checksum() {
    local out_dir="."
    local sums_name="SHA256SUMS"
    local files=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --out-dir) out_dir="$2"; shift 2 ;;
            --sums-name) sums_name="$2"; shift 2 ;;
            --) shift; files+=("$@"); break ;;
            -*) die "checksum: unknown arg: $1" ;;
            *) files+=("$1"); shift ;;
        esac
    done
    [[ ${#files[@]} -gt 0 ]] || die "checksum: at least one file required"
    mkdir -p "$out_dir"

    local sums="${out_dir}/${sums_name}"
    : >"$sums"
    local f base line
    for f in "${files[@]}"; do
        [[ -f "$f" ]] || die "checksum: not a file: $f"
        base=$(basename "$f")
        line=$(sha256_line "$f")
        # Per-artefact detached checksum (basename.sha256)
        # Place next to the artefact when out_dir is its parent; else in out_dir.
        local parent
        parent=$(dirname "$f")
        echo "$line" >"${parent}/${base}.sha256"
        # Combined manifest (kind-specific name avoids browser/WASI clobber on GH Release)
        echo "$line" >>"$sums"
        info "checksum ${base}"
    done
    info "wrote ${sums}"
    echo "sha256sums=${sums}"
}

# ---------------------------------------------------------------------------
# package-browser — VERSION.json + tar.gz + checksums
# ---------------------------------------------------------------------------
cmd_package_browser() {
    local pkg_dir="" tag="" out_dir="." git_sha=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --pkg-dir) pkg_dir="$2"; shift 2 ;;
            --tag) tag="$2"; shift 2 ;;
            --out-dir) out_dir="$2"; shift 2 ;;
            --git-sha) git_sha="$2"; shift 2 ;;
            *) die "package-browser: unknown arg: $1" ;;
        esac
    done
    [[ -n "$pkg_dir" ]] || die "package-browser: --pkg-dir required"
    [[ -d "$pkg_dir" ]] || die "package-browser: --pkg-dir not a directory: $pkg_dir"
    [[ -n "$tag" ]] || die "package-browser: --tag required"
    mkdir -p "$out_dir"

    # Embed version metadata inside the package directory before tarring.
    local ver_out
    ver_out=$(cmd_write_version \
        --kind browser \
        --target wasm32-unknown-unknown \
        --out-dir "$pkg_dir" \
        --tag "$tag" \
        ${git_sha:+--git-sha "$git_sha"} \
        --name "clawft-browser-wasm-${tag}")
    # Also emit a top-level copy next to the tarball for easy release attachment.
    local top_ver="${out_dir}/clawft-browser-wasm-${tag}.VERSION.json"
    cp "${pkg_dir}/VERSION.json" "$top_ver"

    local tarball="${out_dir}/clawft-browser-wasm-${tag}.tar.gz"
    tar -czf "$tarball" -C "$pkg_dir" .
    info "packed ${tarball}"

    local sums_name="clawft-browser-wasm-${tag}.SHA256SUMS"
    # shellcheck disable=SC2046
    eval "$(cmd_checksum --out-dir "$out_dir" --sums-name "$sums_name" "$tarball" "$top_ver")"

    echo "tarball=${tarball}"
    echo "version_json=${top_ver}"
    echo "sha256sums=${sha256sums}"
    # re-echo workspace version for step outputs
    echo "workspace_version=$(workspace_version)"
}

# ---------------------------------------------------------------------------
# package-wasi — VERSION.json + renamed wasm + checksums
# ---------------------------------------------------------------------------
cmd_package_wasi() {
    local wasm="" tag="" out_dir="." git_sha=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --wasm) wasm="$2"; shift 2 ;;
            --tag) tag="$2"; shift 2 ;;
            --out-dir) out_dir="$2"; shift 2 ;;
            --git-sha) git_sha="$2"; shift 2 ;;
            *) die "package-wasi: unknown arg: $1" ;;
        esac
    done
    [[ -n "$wasm" ]] || die "package-wasi: --wasm required"
    [[ -f "$wasm" ]] || die "package-wasi: --wasm not a file: $wasm"
    [[ -n "$tag" ]] || die "package-wasi: --tag required"
    mkdir -p "$out_dir"

    local asset="${out_dir}/clawft-wasm-wasip2-${tag}.wasm"
    cp "$wasm" "$asset"

    local ver_dir
    ver_dir=$(mktemp -d)
    # shellcheck disable=SC2064
    trap "rm -rf '$ver_dir'" RETURN
    cmd_write_version \
        --kind wasi \
        --target wasm32-wasip2 \
        --out-dir "$ver_dir" \
        --tag "$tag" \
        ${git_sha:+--git-sha "$git_sha"} \
        --name "clawft-wasm-wasip2-${tag}" >/dev/null

    local top_ver="${out_dir}/clawft-wasm-wasip2-${tag}.VERSION.json"
    cp "${ver_dir}/VERSION.json" "$top_ver"

    local sums_name="clawft-wasm-wasip2-${tag}.SHA256SUMS"
    # shellcheck disable=SC2046
    eval "$(cmd_checksum --out-dir "$out_dir" --sums-name "$sums_name" "$asset" "$top_ver")"

    echo "asset=${asset}"
    echo "version_json=${top_ver}"
    echo "sha256sums=${sha256sums}"
    echo "workspace_version=$(workspace_version)"
}

# ---------------------------------------------------------------------------
# dispatch
# ---------------------------------------------------------------------------
usage() {
    sed -n '2,45p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

[[ $# -ge 1 ]] || usage 1
cmd="$1"
shift
case "$cmd" in
    write-version)    cmd_write_version "$@" ;;
    checksum)         cmd_checksum "$@" ;;
    package-browser)  cmd_package_browser "$@" ;;
    package-wasi)     cmd_package_wasi "$@" ;;
    -h|--help|help)   usage 0 ;;
    *) die "unknown command: $cmd (try --help)" ;;
esac
