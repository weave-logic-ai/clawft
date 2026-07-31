#!/usr/bin/env bash
# generate-sbom.sh — CycloneDX SBOM for WeftOS release artefacts (WEFT-459).
#
# Uses cargo-cyclonedx (OWASP) so the SBOM describes the binaries we ship
# (weft / weaver / weftos). Output is CycloneDX JSON (spec 1.5) + detached
# SHA-256. Portable to Bash 3.2 (macOS) and Bash 5 (CI).
#
# Note: cargo-cyclonedx 0.5.x walks the full workspace and writes a .cdx.json
# next to every binary. We snapshot paths before the run, harvest only the
# dist-binary BOMs, then delete every newly written file so the tree stays clean.
#
# Usage:
#   scripts/release/generate-sbom.sh generate \
#       [--tag v0.8.0] [--out-dir dist-sbom] [--git-sha abc123]
#
#   scripts/release/generate-sbom.sh check-tool
#
# Env:
#   CARGO_CYCLONEDX_VERSION  pin for `cargo install` (default: 0.5.9)
#
# Outputs (KEY=value on stdout for CI):
#   out_dir=...
#   primary=...
#   assets=...           # space-separated list of all upload paths
#   sha256sums=...
#   workspace_version=...
#   tag=...

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CARGO_CYCLONEDX_VERSION="${CARGO_CYCLONEDX_VERSION:-0.5.9}"

usage() {
  sed -n '2,28p' "$0" | sed 's/^# \?//'
  exit "${1:-0}"
}

# Map cargo-cyclonedx --describe binaries basename (without .cdx.json) → asset stem.
dist_stem_for_bin_key() {
  case "$1" in
    weft_bin) echo "weft-cli" ;;
    weaver_bin) echo "weaver" ;;
    weftos_bin) echo "weftos" ;;
    *) echo "" ;;
  esac
}

workspace_version() {
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY'
import re, pathlib
text = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"', text, re.M | re.S)
if m:
    print(m.group(1))
else:
    raise SystemExit("could not parse workspace.package.version from Cargo.toml")
PY
  else
    cargo metadata --no-deps --format-version 1 \
      | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -1
  fi
}

ensure_cargo_cyclonedx() {
  if cargo cyclonedx --version >/dev/null 2>&1; then
    return 0
  fi
  echo "cargo-cyclonedx not found; installing ${CARGO_CYCLONEDX_VERSION}..." >&2
  cargo install cargo-cyclonedx --locked --version "$CARGO_CYCLONEDX_VERSION"
}

normalize_tag() {
  local t="$1"
  if [ -z "$t" ]; then
    t="v$(workspace_version)"
  fi
  case "$t" in
    v*) ;;
    *) t="v${t}" ;;
  esac
  printf '%s' "$t"
}

# Write newline-separated list of SBOM-like files cargo-cyclonedx may create.
list_sbom_files() {
  find . \
    \( -name '*.cdx.json' -o -name '*.cdx.xml' -o -name 'bom.json' -o -name 'bom.xml' \) \
    -not -path './target/*' \
    -not -path './.git/*' \
    2>/dev/null || true
}

cmd_check_tool() {
  ensure_cargo_cyclonedx
  cargo cyclonedx --version
}

cmd_generate() {
  local tag="" out_dir="dist-sbom" git_sha="${GITHUB_SHA:-}"
  while [ $# -gt 0 ]; do
    case "$1" in
      --tag) tag="${2:-}"; shift 2 ;;
      --out-dir) out_dir="${2:-}"; shift 2 ;;
      --git-sha) git_sha="${2:-}"; shift 2 ;;
      -h|--help) usage 0 ;;
      *) echo "unknown arg: $1" >&2; usage 1 ;;
    esac
  done

  ensure_cargo_cyclonedx

  local ws_ver
  ws_ver="$(workspace_version)"
  tag="$(normalize_tag "$tag")"
  if [ -z "$git_sha" ]; then
    git_sha="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
  fi

  mkdir -p "$out_dir"
  local abs_out
  abs_out="$(cd "$out_dir" && pwd)"

  local before_file new_file
  before_file="$(mktemp)"
  new_file="$(mktemp)"
  list_sbom_files | sort >"$before_file"

  # Single workspace pass: one BOM per binary target.
  # --describe binaries is mutually exclusive with --override-filename in 0.5.x.
  echo "running cargo cyclonedx (workspace binaries, CycloneDX 1.5)..." >&2
  cargo cyclonedx \
    --format json \
    --spec-version 1.5 \
    --describe binaries \
    --all \
    --quiet

  list_sbom_files | sort >"$new_file"

  # Diff: files present after but not before.
  local -a new_files=()
  local line
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    if ! grep -Fxq -- "$line" "$before_file"; then
      new_files+=("$line")
    fi
  done <"$new_file"

  cleanup_new() {
    local p
    for p in "${new_files[@]+"${new_files[@]}"}"; do
      rm -f "$p" 2>/dev/null || true
    done
    rm -f "$before_file" "$new_file" 2>/dev/null || true
  }
  trap cleanup_new EXIT

  if [ "${#new_files[@]}" -eq 0 ]; then
    echo "::error::cargo-cyclonedx produced no new SBOM files" >&2
    exit 1
  fi

  local -a produced=()
  local stem base dest found=0 f
  for f in "${new_files[@]}"; do
    base="$(basename "$f")"
    case "$base" in
      *_bin.cdx.json) ;;
      *) continue ;;
    esac
    stem="$(dist_stem_for_bin_key "${base%.cdx.json}")"
    if [ -z "$stem" ]; then
      continue
    fi
    dest="${abs_out}/${stem}-${tag}.cdx.json"
    cp "$f" "$dest"
    produced+=("$dest")
    found=$((found + 1))
    echo "harvested ${base} -> $(basename "$dest")" >&2
  done

  if [ "$found" -eq 0 ]; then
    echo "::error::no dist-package binary SBOMs found (expected weft_bin / weaver_bin / weftos_bin)" >&2
    printf '  %s\n' "${new_files[@]}" >&2
    exit 1
  fi

  # Primary = weft-cli when present, else first harvested.
  local primary=""
  for f in "${produced[@]}"; do
    case "$(basename "$f")" in
      weft-cli-*) primary="$f"; break ;;
    esac
  done
  if [ -z "$primary" ]; then
    primary="${produced[0]}"
  fi

  # If weftos_bin was not harvested (unlikely), still publish a stable
  # weftos-<tag>.cdx.json alias of the primary (weft-cli) for scanners.
  local alias="${abs_out}/weftos-${tag}.cdx.json"
  if [ ! -f "$alias" ]; then
    cp "$primary" "$alias"
    produced+=("$alias")
  fi

  local meta="${abs_out}/weftos-${tag}.sbom-meta.json"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$meta" "$tag" "$git_sha" "$ws_ver" "${produced[@]}" <<'PY'
import json, sys, pathlib, hashlib
meta_path, tag, git_sha, ws_ver, *assets = sys.argv[1:]
files = []
for p in assets:
    data = pathlib.Path(p).read_bytes()
    files.append({
        "name": pathlib.Path(p).name,
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    })
doc = {
    "schema": "weftos.sbom-meta/v1",
    "tag": tag,
    "git_sha": git_sha,
    "workspace_version": ws_ver,
    "format": "CycloneDX",
    "spec_version": "1.5",
    "generator": "cargo-cyclonedx",
    "files": files,
}
pathlib.Path(meta_path).write_text(json.dumps(doc, indent=2) + "\n")
PY
  else
    printf '{\n  "tag": "%s",\n  "git_sha": "%s",\n  "workspace_version": "%s"\n}\n' \
      "$tag" "$git_sha" "$ws_ver" >"$meta"
  fi
  produced+=("$meta")

  local sums="${abs_out}/weftos-${tag}.cdx.SHA256SUMS"
  : >"$sums"
  local base_name
  for f in "${produced[@]}"; do
    base_name="$(basename "$f")"
    if command -v sha256sum >/dev/null 2>&1; then
      (cd "$abs_out" && sha256sum "$base_name" | tee -a "$sums" >"${f}.sha256")
    else
      (cd "$abs_out" && shasum -a 256 "$base_name" | tee -a "$sums" >"${f}.sha256")
    fi
    produced+=("${f}.sha256")
  done
  produced+=("$sums")

  cleanup_new
  trap - EXIT

  local assets_joined=""
  for f in "${produced[@]}"; do
    if [ -z "$assets_joined" ]; then
      assets_joined="$f"
    else
      assets_joined="${assets_joined} $f"
    fi
  done

  # Quote values for safe `eval "$(…)"` in CI. Unquoted space-separated
  # assets=a b c makes bash try to execute `b` and `c` (Permission denied
  # on .cdx.json during Release SBOM on v0.8.0).
  printf "out_dir=%q\n" "${abs_out}"
  printf "primary=%q\n" "${primary}"
  printf "assets=%q\n" "${assets_joined}"
  printf "sha256sums=%q\n" "${sums}"
  printf "workspace_version=%q\n" "${ws_ver}"
  printf "tag=%q\n" "${tag}"
}

main() {
  local cmd="${1:-}"
  if [ $# -gt 0 ]; then
    shift
  fi
  case "$cmd" in
    generate) cmd_generate "$@" ;;
    check-tool) cmd_check_tool "$@" ;;
    -h|--help|help|"") usage 0 ;;
    *) echo "unknown command: $cmd" >&2; usage 1 ;;
  esac
}

main "$@"
