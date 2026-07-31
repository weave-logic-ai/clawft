#!/usr/bin/env bash
# cdn-snapshot.sh — SHA-stamped snapshots of the rolling cdn-assets release.
#
# WEFT-454: every upload to the rolling `cdn-assets` GitHub Release also
# writes clawft_wasm-{sha}.wasm / clawft_wasm-{sha}.js / weftos-docs-{sha}.rvf
# so a bad WASM/KB clobber can be rolled back. The rolling filenames stay
# the live "latest" pointers; SHA stamps are retained for the last N
# uploads (default 10).
#
# Usage:
#   # Stage SHA-stamped copies + manifest (no network)
#   scripts/release/cdn-snapshot.sh prepare \
#       --sha abc123def456 \
#       --wasm-dir browser-pkg \
#       --kb weftos-docs.rvf \
#       --out-dir cdn-snap
#
#   # Upload rolling + SHA stamps, then prune old stamps (CI path)
#   scripts/release/cdn-snapshot.sh publish \
#       --tag cdn-assets \
#       --sha abc123def456 \
#       --wasm-dir browser-pkg \
#       --kb weftos-docs.rvf \
#       [--tarball browser-wasm-pkg.tar.gz] \
#       [--retain 10] [--skip-wasm] [--dry-run]
#
#   # Keep only the newest N SHA groups on the release
#   scripts/release/cdn-snapshot.sh prune \
#       --tag cdn-assets [--retain 10] [--dry-run]
#
#   # Restore rolling filenames from a SHA snapshot (ops rollback)
#   scripts/release/cdn-snapshot.sh rollback \
#       --tag cdn-assets --sha abc123def456 [--dry-run]
#
#   # List retained SHA snapshots on the release
#   scripts/release/cdn-snapshot.sh list --tag cdn-assets
#
# Env:
#   GH_TOKEN / GITHUB_TOKEN  — required for publish/prune/rollback/list
#   CDN_RETAIN               — default retain count (default 10)
#
# Outputs (KEY=value lines for CI):
#   sha_short=...
#   manifest=...
#   snapshot_wasm=...
#   snapshot_js=...
#   snapshot_kb=...

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

die() { echo "error: $*" >&2; exit 1; }
info() { echo "→ $*" >&2; }

DEFAULT_TAG="cdn-assets"
DEFAULT_RETAIN="${CDN_RETAIN:-10}"
# AC + audit: clawft_wasm-{sha}.wasm (not clawft_wasm_bg-{sha}.wasm)
ROLLING_WASM="clawft_wasm_bg.wasm"
ROLLING_JS="clawft_wasm.js"
ROLLING_KB="weftos-docs.rvf"
MANIFEST_NAME="cdn-manifest.json"

iso_now() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

normalize_sha() {
    local raw="$1"
    raw=$(echo "$raw" | tr -d '[:space:]' | tr '[:upper:]' '[:lower:]')
    [[ "$raw" =~ ^[0-9a-f]{7,40}$ ]] || die "invalid git sha: $raw (want 7–40 hex)"
    # Prefer 12-char short (matches package-wasm-artifact.sh)
    if [[ ${#raw} -ge 12 ]]; then
        echo "${raw:0:12}"
    else
        echo "$raw"
    fi
}

full_sha_hint() {
    local raw="$1"
    raw=$(echo "$raw" | tr -d '[:space:]' | tr '[:upper:]' '[:lower:]')
    if [[ ${#raw} -eq 40 ]]; then
        echo "$raw"
    elif git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git rev-parse HEAD 2>/dev/null || echo "$raw"
    else
        echo "$raw"
    fi
}

require_gh() {
    command -v gh >/dev/null 2>&1 || die "gh CLI required (https://cli.github.com/)"
    if [[ -z "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]]; then
        die "GH_TOKEN or GITHUB_TOKEN required for GitHub Release operations"
    fi
    # Prefer GH_TOKEN for gh
    export GH_TOKEN="${GH_TOKEN:-$GITHUB_TOKEN}"
}

snapshot_names() {
    local sha="$1"
    echo "wasm=clawft_wasm-${sha}.wasm"
    echo "js=clawft_wasm-${sha}.js"
    echo "kb=weftos-docs-${sha}.rvf"
}

# ---------------------------------------------------------------------------
# prepare — copy rolling inputs to SHA-stamped names + write manifest
# ---------------------------------------------------------------------------
cmd_prepare() {
    local sha="" wasm_dir="" kb="" out_dir="cdn-snap" tarball="" skip_wasm=0
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --sha) sha="$2"; shift 2 ;;
            --wasm-dir) wasm_dir="$2"; shift 2 ;;
            --kb) kb="$2"; shift 2 ;;
            --out-dir) out_dir="$2"; shift 2 ;;
            --tarball) tarball="$2"; shift 2 ;;
            --skip-wasm) skip_wasm=1; shift ;;
            *) die "prepare: unknown arg: $1" ;;
        esac
    done
    [[ -n "$sha" ]] || die "prepare: --sha required"
    [[ -n "$kb" ]] || die "prepare: --kb required"
    [[ -f "$kb" ]] || die "prepare: KB not found: $kb"
    local sha_short sha_full
    sha_short=$(normalize_sha "$sha")
    sha_full=$(full_sha_hint "$sha")

    mkdir -p "$out_dir"
    local snap_wasm="clawft_wasm-${sha_short}.wasm"
    local snap_js="clawft_wasm-${sha_short}.js"
    local snap_kb="weftos-docs-${sha_short}.rvf"
    local published
    published=$(iso_now)

    if [[ "$skip_wasm" -eq 0 ]]; then
        [[ -n "$wasm_dir" ]] || die "prepare: --wasm-dir required (or --skip-wasm)"
        [[ -f "$wasm_dir/$ROLLING_WASM" ]] || die "prepare: missing $wasm_dir/$ROLLING_WASM"
        [[ -f "$wasm_dir/$ROLLING_JS" ]] || die "prepare: missing $wasm_dir/$ROLLING_JS"
        cp "$wasm_dir/$ROLLING_WASM" "$out_dir/$snap_wasm"
        cp "$wasm_dir/$ROLLING_JS" "$out_dir/$snap_js"
        # Also stage rolling names for a single upload tree
        cp "$wasm_dir/$ROLLING_WASM" "$out_dir/$ROLLING_WASM"
        cp "$wasm_dir/$ROLLING_JS" "$out_dir/$ROLLING_JS"
        info "staged WASM snapshot $snap_wasm"
        info "staged JS snapshot   $snap_js"
    fi

    cp "$kb" "$out_dir/$snap_kb"
    cp "$kb" "$out_dir/$ROLLING_KB"
    info "staged KB snapshot  $snap_kb"

    if [[ -n "$tarball" && -f "$tarball" ]]; then
        local snap_tar="browser-wasm-pkg-${sha_short}.tar.gz"
        cp "$tarball" "$out_dir/$snap_tar"
        cp "$tarball" "$out_dir/$(basename "$tarball")"
        info "staged tarball snapshot $snap_tar"
    fi

    local manifest="$out_dir/$MANIFEST_NAME"
    # Pure bash JSON — controlled inputs only
    cat >"$manifest" <<EOF
{
  "schema": "weftos.cdn-assets.v1",
  "tag": "${DEFAULT_TAG}",
  "git_sha": "${sha_full}",
  "git_sha_short": "${sha_short}",
  "published_at": "${published}",
  "skip_wasm": $([[ "$skip_wasm" -eq 1 ]] && echo true || echo false),
  "rolling": {
    "wasm": "${ROLLING_WASM}",
    "js": "${ROLLING_JS}",
    "kb": "${ROLLING_KB}"
  },
  "snapshot": {
    "wasm": $([[ "$skip_wasm" -eq 0 ]] && echo "\"${snap_wasm}\"" || echo null),
    "js": $([[ "$skip_wasm" -eq 0 ]] && echo "\"${snap_js}\"" || echo null),
    "kb": "${snap_kb}"
  },
  "retain_default": ${DEFAULT_RETAIN},
  "notes": "Rolling filenames are the live docs-site pointers. SHA-stamped siblings are retained for rollback (WEFT-454)."
}
EOF
    info "wrote $manifest"

    echo "sha_short=${sha_short}"
    echo "manifest=${manifest}"
    if [[ "$skip_wasm" -eq 0 ]]; then
        echo "snapshot_wasm=${out_dir}/${snap_wasm}"
        echo "snapshot_js=${out_dir}/${snap_js}"
    fi
    echo "snapshot_kb=${out_dir}/${snap_kb}"
    echo "out_dir=${out_dir}"
}

# ---------------------------------------------------------------------------
# publish — prepare + gh release upload (rolling + stamps) + prune
# ---------------------------------------------------------------------------
cmd_publish() {
    local sha="" wasm_dir="" kb="" tag="$DEFAULT_TAG" retain="$DEFAULT_RETAIN"
    local tarball="" skip_wasm=0 dry_run=0 out_dir=""
    local work_dir
    work_dir=$(mktemp -d "${TMPDIR:-/tmp}/cdn-snap.XXXXXX")
    # shellcheck disable=SC2064
    trap "rm -rf '$work_dir'" EXIT

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --sha) sha="$2"; shift 2 ;;
            --wasm-dir) wasm_dir="$2"; shift 2 ;;
            --kb) kb="$2"; shift 2 ;;
            --tag) tag="$2"; shift 2 ;;
            --retain) retain="$2"; shift 2 ;;
            --tarball) tarball="$2"; shift 2 ;;
            --skip-wasm) skip_wasm=1; shift ;;
            --dry-run) dry_run=1; shift ;;
            --out-dir) out_dir="$2"; shift 2 ;;
            *) die "publish: unknown arg: $1" ;;
        esac
    done
    [[ -n "$sha" ]] || die "publish: --sha required"
    [[ -n "$kb" ]] || die "publish: --kb required"
    [[ "$retain" =~ ^[0-9]+$ ]] || die "publish: --retain must be a non-negative integer"
    out_dir="${out_dir:-$work_dir}"

    local prep_args=(--sha "$sha" --kb "$kb" --out-dir "$out_dir")
    if [[ "$skip_wasm" -eq 1 ]]; then
        prep_args+=(--skip-wasm)
    else
        prep_args+=(--wasm-dir "$wasm_dir")
    fi
    if [[ -n "$tarball" ]]; then
        prep_args+=(--tarball "$tarball")
    fi
    # Capture KEY=value without polluting stdout of publish too early
    local prep_out
    prep_out=$(cmd_prepare "${prep_args[@]}")
    echo "$prep_out"
    local sha_short
    sha_short=$(echo "$prep_out" | sed -n 's/^sha_short=//p' | head -1)

    # Build upload file list: rolling + snapshot + manifest
    local files=()
    files+=("$out_dir/$MANIFEST_NAME")
    files+=("$out_dir/$ROLLING_KB")
    files+=("$out_dir/weftos-docs-${sha_short}.rvf")
    if [[ "$skip_wasm" -eq 0 ]]; then
        files+=("$out_dir/$ROLLING_WASM")
        files+=("$out_dir/$ROLLING_JS")
        files+=("$out_dir/clawft_wasm-${sha_short}.wasm")
        files+=("$out_dir/clawft_wasm-${sha_short}.js")
        if [[ -n "$tarball" && -f "$out_dir/$(basename "$tarball")" ]]; then
            files+=("$out_dir/$(basename "$tarball")")
        fi
        if [[ -f "$out_dir/browser-wasm-pkg-${sha_short}.tar.gz" ]]; then
            files+=("$out_dir/browser-wasm-pkg-${sha_short}.tar.gz")
        fi
    fi

    for f in "${files[@]}"; do
        [[ -f "$f" ]] || die "publish: staged file missing: $f"
    done

    if [[ "$dry_run" -eq 1 ]]; then
        info "dry-run: would upload to release tag '$tag':"
        for f in "${files[@]}"; do
            echo "  $(basename "$f") ($(du -h "$f" | cut -f1))"
        done
        info "dry-run: would prune retain=$retain"
        return 0
    fi

    require_gh
    if ! gh release view "$tag" >/dev/null 2>&1; then
        info "creating release $tag"
        gh release create "$tag" \
            --title "CDN Assets (latest)" \
            --notes "Rolling release for docs site CDN assets. SHA-stamped snapshots retained for rollback (WEFT-454)." \
            --prerelease
    fi

    info "uploading ${#files[@]} assets to $tag (rolling + sha=${sha_short})"
    # --clobber only affects same-named rolling assets; SHA stamps are unique
    gh release upload "$tag" "${files[@]}" --clobber

    cmd_prune --tag "$tag" --retain "$retain"
    info "publish complete sha=${sha_short} tag=$tag"
}

# ---------------------------------------------------------------------------
# prune — drop oldest SHA snapshot groups beyond --retain
# ---------------------------------------------------------------------------
cmd_prune() {
    local tag="$DEFAULT_TAG" retain="$DEFAULT_RETAIN" dry_run=0
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tag) tag="$2"; shift 2 ;;
            --retain) retain="$2"; shift 2 ;;
            --dry-run) dry_run=1; shift ;;
            *) die "prune: unknown arg: $1" ;;
        esac
    done
    [[ "$retain" =~ ^[0-9]+$ ]] || die "prune: --retain must be a non-negative integer"

    if [[ "$dry_run" -eq 0 ]]; then
        require_gh
    else
        command -v gh >/dev/null 2>&1 || die "gh CLI required"
        export GH_TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
        if [[ -z "${GH_TOKEN:-}" ]]; then
            info "dry-run prune: no GH_TOKEN — skipping remote list"
            return 0
        fi
    fi

    # asset name patterns for a SHA group:
    #   clawft_wasm-{sha}.wasm | clawft_wasm-{sha}.js | weftos-docs-{sha}.rvf
    #   browser-wasm-pkg-{sha}.tar.gz
    local json
    if ! json=$(gh release view "$tag" --json assets 2>/dev/null); then
        die "prune: cannot view release $tag"
    fi

    # Extract unique SHAs that have at least one stamped asset, ordered by
    # newest createdAt among that SHA's assets.
    # Prefer python for portable JSON; fall back to pure bash+grep if needed.
    local ordered_shas=""
    if command -v python3 >/dev/null 2>&1; then
        ordered_shas=$(CDN_JSON="$json" python3 - <<'PY'
import json, os, re
from collections import defaultdict
data = json.loads(os.environ["CDN_JSON"])
pat = re.compile(
    r"^(?:clawft_wasm|weftos-docs|browser-wasm-pkg)-([0-9a-f]{7,40})\."
    r"(?:wasm|js|rvf|tar\.gz)$"
)
best = {}  # sha -> max createdAt
for a in data.get("assets") or []:
    name = a.get("name") or ""
    m = pat.match(name)
    if not m:
        continue
    sha = m.group(1)
    ts = a.get("createdAt") or ""
    if sha not in best or ts > best[sha]:
        best[sha] = ts
# newest first
for sha, _ in sorted(best.items(), key=lambda kv: kv[1], reverse=True):
    print(sha)
PY
)
    else
        die "prune: python3 required to parse gh release JSON"
    fi

    if [[ -z "$ordered_shas" ]]; then
        info "prune: no SHA-stamped assets on $tag"
        return 0
    fi

    local -a shas=()
    while IFS= read -r line; do
        [[ -n "$line" ]] && shas+=("$line")
    done <<<"$ordered_shas"

    local total=${#shas[@]}
    info "prune: found $total SHA snapshot group(s); retain=$retain"

    if [[ "$total" -le "$retain" ]]; then
        info "prune: nothing to delete"
        return 0
    fi

    local -a drop=("${shas[@]:$retain}")
    local sha name
    local -a to_delete=()
    # Re-read asset names for drop SHAs
    local all_names
    all_names=$(CDN_JSON="$json" python3 - <<'PY'
import json, os, re
data = json.loads(os.environ["CDN_JSON"])
pat = re.compile(
    r"^(?:clawft_wasm|weftos-docs|browser-wasm-pkg)-([0-9a-f]{7,40})\."
    r"(?:wasm|js|rvf|tar\.gz)$"
)
for a in data.get("assets") or []:
    name = a.get("name") or ""
    m = pat.match(name)
    if m:
        print(f"{m.group(1)}\t{name}")
PY
)
    for sha in "${drop[@]}"; do
        while IFS=$'\t' read -r s name; do
            if [[ "$s" == "$sha" ]]; then
                to_delete+=("$name")
            fi
        done <<<"$all_names"
    done

    if [[ ${#to_delete[@]} -eq 0 ]]; then
        info "prune: no asset files mapped for drop set"
        return 0
    fi

    info "prune: deleting ${#to_delete[@]} asset(s) from ${#drop[@]} old SHA group(s)"
    for name in "${to_delete[@]}"; do
        if [[ "$dry_run" -eq 1 ]]; then
            echo "  would delete: $name"
        else
            info "deleting $name"
            gh release delete-asset "$tag" "$name" --yes
        fi
    done
}

# ---------------------------------------------------------------------------
# rollback — copy SHA snapshot over rolling names
# ---------------------------------------------------------------------------
cmd_rollback() {
    local tag="$DEFAULT_TAG" sha="" dry_run=0
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tag) tag="$2"; shift 2 ;;
            --sha) sha="$2"; shift 2 ;;
            --dry-run) dry_run=1; shift ;;
            *) die "rollback: unknown arg: $1" ;;
        esac
    done
    [[ -n "$sha" ]] || die "rollback: --sha required"
    local sha_short
    sha_short=$(normalize_sha "$sha")
    require_gh

    local work
    work=$(mktemp -d "${TMPDIR:-/tmp}/cdn-rollback.XXXXXX")
    # shellcheck disable=SC2064
    trap "rm -rf '$work'" EXIT

    local snap_wasm="clawft_wasm-${sha_short}.wasm"
    local snap_js="clawft_wasm-${sha_short}.js"
    local snap_kb="weftos-docs-${sha_short}.rvf"

    info "downloading snapshots for sha=${sha_short} from $tag"
    # Download by pattern: gh release download filters by pattern
    if ! gh release download "$tag" --dir "$work" --pattern "$snap_wasm" 2>/dev/null; then
        die "rollback: missing $snap_wasm on $tag"
    fi
    if ! gh release download "$tag" --dir "$work" --pattern "$snap_js" 2>/dev/null; then
        die "rollback: missing $snap_js on $tag"
    fi
    if ! gh release download "$tag" --dir "$work" --pattern "$snap_kb" 2>/dev/null; then
        die "rollback: missing $snap_kb on $tag"
    fi

    cp "$work/$snap_wasm" "$work/$ROLLING_WASM"
    cp "$work/$snap_js" "$work/$ROLLING_JS"
    cp "$work/$snap_kb" "$work/$ROLLING_KB"

    # Refresh manifest to point at restored SHA
    local published
    published=$(iso_now)
    cat >"$work/$MANIFEST_NAME" <<EOF
{
  "schema": "weftos.cdn-assets.v1",
  "tag": "${tag}",
  "git_sha": "${sha_short}",
  "git_sha_short": "${sha_short}",
  "published_at": "${published}",
  "restored_from_snapshot": true,
  "rolling": {
    "wasm": "${ROLLING_WASM}",
    "js": "${ROLLING_JS}",
    "kb": "${ROLLING_KB}"
  },
  "snapshot": {
    "wasm": "${snap_wasm}",
    "js": "${snap_js}",
    "kb": "${snap_kb}"
  },
  "notes": "Rolling filenames restored from SHA snapshot via cdn-snapshot.sh rollback (WEFT-454)."
}
EOF

    if [[ "$dry_run" -eq 1 ]]; then
        info "dry-run: would clobber rolling assets with sha=${sha_short}"
        ls -lh "$work"
        return 0
    fi

    info "uploading restored rolling assets (sha=${sha_short})"
    gh release upload "$tag" \
        "$work/$ROLLING_WASM" \
        "$work/$ROLLING_JS" \
        "$work/$ROLLING_KB" \
        "$work/$MANIFEST_NAME" \
        --clobber
    info "rollback complete: rolling pointers now match sha=${sha_short}"
    echo "sha_short=${sha_short}"
    echo "tag=${tag}"
}

# ---------------------------------------------------------------------------
# list — print retained SHA groups
# ---------------------------------------------------------------------------
cmd_list() {
    local tag="$DEFAULT_TAG"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tag) tag="$2"; shift 2 ;;
            *) die "list: unknown arg: $1" ;;
        esac
    done
    require_gh
    local json
    json=$(gh release view "$tag" --json assets) || die "list: cannot view $tag"
    CDN_JSON="$json" python3 - <<'PY'
import json, os, re
from collections import defaultdict
data = json.loads(os.environ["CDN_JSON"])
pat = re.compile(
    r"^(?:clawft_wasm|weftos-docs|browser-wasm-pkg)-([0-9a-f]{7,40})\."
    r"(?:wasm|js|rvf|tar\.gz)$"
)
groups = defaultdict(lambda: {"files": [], "createdAt": ""})
for a in data.get("assets") or []:
    name = a.get("name") or ""
    m = pat.match(name)
    if not m:
        continue
    sha = m.group(1)
    groups[sha]["files"].append(name)
    ts = a.get("createdAt") or ""
    if ts > groups[sha]["createdAt"]:
        groups[sha]["createdAt"] = ts
if not groups:
    print("(no SHA-stamped snapshots)")
else:
    for sha, meta in sorted(groups.items(), key=lambda kv: kv[1]["createdAt"], reverse=True):
        print(f"{sha}\t{meta['createdAt']}\t{len(meta['files'])} files")
PY
}

usage() {
    sed -n '2,45p' "$0" | sed 's/^# \?//'
}

main() {
    local cmd="${1:-}"
    shift || true
    case "$cmd" in
        prepare)  cmd_prepare "$@" ;;
        publish)  cmd_publish "$@" ;;
        prune)    cmd_prune "$@" ;;
        rollback) cmd_rollback "$@" ;;
        list)     cmd_list "$@" ;;
        -h|--help|help|"") usage ;;
        *) die "unknown command: $cmd (try: prepare|publish|prune|rollback|list)" ;;
    esac
}

main "$@"
