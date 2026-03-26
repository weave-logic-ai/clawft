#!/usr/bin/env bash
# weave init — Initialize clawft+WeftOS development environment
#
# Copies agents/skills to .claude/skills/ and prepares the workspace.
# Run from the project root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AGENTS_DIR="$PROJECT_ROOT/agents"
CLAUDE_SKILLS="$PROJECT_ROOT/.claude/skills"

# Parse args
FORCE=false
SKILLS_ONLY=false
ANALYZE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --force|-f) FORCE=true; shift ;;
        --skills) SKILLS_ONLY=true; shift ;;
        --analyze) ANALYZE=true; shift ;;
        --help|-h)
            echo "Usage: weave init [options]"
            echo ""
            echo "Options:"
            echo "  --force, -f    Overwrite existing skills"
            echo "  --skills       Only install skills (skip other setup)"
            echo "  --analyze      Run initial ECC analysis after setup"
            echo "  --help, -h     Show this help"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== weave init ==="
echo ""

# 1. Create .claude/skills/ directory
mkdir -p "$CLAUDE_SKILLS"
echo "  Created $CLAUDE_SKILLS"

# 2. Copy all agents to .claude/skills/
for agent_dir in "$AGENTS_DIR"/*/; do
    agent_name=$(basename "$agent_dir")
    target="$CLAUDE_SKILLS/$agent_name"

    if [ -d "$target" ] && [ "$FORCE" != "true" ]; then
        echo "  Skipping $agent_name (exists, use --force to overwrite)"
    else
        mkdir -p "$target"
        cp -r "$agent_dir"* "$target/"
        echo "  Installed $agent_name"
    fi
done

if [ "$SKILLS_ONLY" = "true" ]; then
    echo ""
    echo "Skills installed. Done."
    exit 0
fi

# 3. Verify build tools
echo ""
echo "--- Checking build tools ---"

if [ -f "$PROJECT_ROOT/scripts/build.sh" ]; then
    echo "  build.sh: found"
else
    echo "  build.sh: MISSING"
fi

if command -v cargo &>/dev/null; then
    echo "  cargo: $(cargo --version)"
else
    echo "  cargo: NOT FOUND"
fi

if [ -f "$PROJECT_ROOT/scripts/k6-gate.sh" ]; then
    echo "  k6-gate.sh: found"
else
    echo "  k6-gate.sh: not found"
fi

# 4. Quick compile check
echo ""
echo "--- Quick compile check ---"
if [ -f "$PROJECT_ROOT/scripts/build.sh" ]; then
    "$PROJECT_ROOT/scripts/build.sh" check 2>&1 | tail -3
else
    cargo check -p clawft-kernel --features native 2>&1 | tail -3
fi

# 5. Show installed skills
echo ""
echo "--- Installed Skills ---"
for skill_dir in "$CLAUDE_SKILLS"/*/; do
    skill_name=$(basename "$skill_dir")
    # Count .md files
    md_count=$(find "$skill_dir" -name "*.md" | wc -l)
    echo "  $skill_name ($md_count skill files)"
done

# 6. Optional: initial ECC analysis
if [ "$ANALYZE" = "true" ]; then
    echo ""
    echo "--- Initial ECC Analysis ---"
    echo "  (Would run Weaver analysis — not yet implemented)"
    echo "  Future: weaver ecc analyze $PROJECT_ROOT --context 'clawft+WeftOS kernel'"
fi

echo ""
echo "=== weave init complete ==="
echo ""
echo "Available skills (use in Claude Code conversations):"
for skill_dir in "$CLAUDE_SKILLS"/*/; do
    skill_name=$(basename "$skill_dir")
    for md in "$skill_dir"*.md; do
        [ -f "$md" ] && echo "  /$skill_name/$(basename "$md" .md)"
    done
done
