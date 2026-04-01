#!/usr/bin/env bash
# Generate rustdoc API reference and copy into Fumadocs public/ for serving at /api/
#
# Usage:
#   scripts/generate-api-docs.sh          # Generate and copy
#   scripts/generate-api-docs.sh --clean  # Remove generated docs
#
# The output lands in docs/src/public/api/ which Next.js serves as static files.
# Access at: https://weftos.weavelogic.ai/api/weftos/index.html

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOC_OUTPUT="$REPO_ROOT/target/doc"
PUBLIC_API="$REPO_ROOT/docs/src/public/api"

# Published crates to document
CRATES=(
  weftos
  clawft-kernel
  clawft-core
  clawft-types
  clawft-platform
  clawft-plugin
  clawft-llm
  exo-resource-tree
)

if [[ "${1:-}" == "--clean" ]]; then
  echo "Removing $PUBLIC_API"
  rm -rf "$PUBLIC_API"
  exit 0
fi

echo "=== Generating rustdoc for ${#CRATES[@]} crates ==="

CRATE_FLAGS=""
for crate in "${CRATES[@]}"; do
  CRATE_FLAGS="$CRATE_FLAGS -p $crate"
done

cargo doc --no-deps $CRATE_FLAGS

echo "=== Copying to $PUBLIC_API ==="

# Clean previous output
rm -rf "$PUBLIC_API"
mkdir -p "$PUBLIC_API"

# Copy all doc output (HTML, CSS, JS, search index)
cp -r "$DOC_OUTPUT"/* "$PUBLIC_API/"

# Remove .lock file if present
rm -f "$PUBLIC_API/.lock"

# Remove source viewer (11MB+ — users can read source on GitHub)
rm -rf "$PUBLIC_API/src"
rm -f "$PUBLIC_API/src-files.js"

# Create a redirect index.html at /api/ pointing to /api/weftos/
cat > "$PUBLIC_API/index.html" <<'REDIRECT'
<!DOCTYPE html>
<html>
<head>
  <meta http-equiv="refresh" content="0; url=weftos/index.html">
  <title>WeftOS API Reference</title>
</head>
<body>
  <p>Redirecting to <a href="weftos/index.html">WeftOS API Reference</a>...</p>
</body>
</html>
REDIRECT

# Count output
HTML_COUNT=$(find "$PUBLIC_API" -name "*.html" | wc -l)
echo "=== Done: $HTML_COUNT HTML pages in $PUBLIC_API ==="
echo "Access at: /api/weftos/index.html"
