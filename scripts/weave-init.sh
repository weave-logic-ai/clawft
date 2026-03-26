#!/usr/bin/env bash
# weave init — Initialize clawft+WeftOS development environment
#
# Copies agents to .claude/agents/ and skills to .claude/skills/,
# then prepares the workspace.
# Run from the project root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AGENTS_DIR="$PROJECT_ROOT/agents"
CLAUDE_SKILLS="$PROJECT_ROOT/.claude/skills"
CLAUDE_AGENTS="$PROJECT_ROOT/.claude/agents"

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
            echo "  --force, -f    Overwrite existing agents and skills"
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

# 1. Create target directories
mkdir -p "$CLAUDE_SKILLS"
mkdir -p "$CLAUDE_AGENTS"
echo "  Created $CLAUDE_SKILLS"
echo "  Created $CLAUDE_AGENTS"

# 2. Copy agent teams to .claude/agents/ (directories containing agent .md files)
#    Agent directories: weftos/ (the team), weftos-ecc/ (also has agent-style content)
AGENT_DIRS=("weftos" "weftos-ecc")

echo ""
echo "--- Installing Agents ---"
for agent_team in "${AGENT_DIRS[@]}"; do
    source_dir="$AGENTS_DIR/$agent_team"
    target="$CLAUDE_AGENTS/$agent_team"

    if [ ! -d "$source_dir" ]; then
        echo "  Skipping $agent_team (source not found)"
        continue
    fi

    if [ -d "$target" ] && [ "$FORCE" != "true" ]; then
        echo "  Skipping $agent_team (exists, use --force to overwrite)"
    else
        mkdir -p "$target"
        cp -r "$source_dir"/* "$target/"
        agent_count=$(find "$target" -name "*.md" | wc -l)
        echo "  Installed $agent_team ($agent_count agent files)"
    fi
done

# 3. Copy all agent dirs to .claude/skills/ (skills = reference knowledge)
echo ""
echo "--- Installing Skills ---"
for agent_dir in "$AGENTS_DIR"/*/; do
    agent_name=$(basename "$agent_dir")
    target="$CLAUDE_SKILLS/$agent_name"

    if [ -d "$target" ] && [ "$FORCE" != "true" ]; then
        echo "  Skipping $agent_name (exists, use --force to overwrite)"
    else
        mkdir -p "$target"
        cp -r "$agent_dir"* "$target/"
        echo "  Installed skill: $agent_name"
    fi
done

if [ "$SKILLS_ONLY" = "true" ]; then
    echo ""
    echo "Skills installed. Done."
    exit 0
fi

# 4. Verify build tools
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

# 5. Quick compile check
echo ""
echo "--- Quick compile check ---"
if [ -f "$PROJECT_ROOT/scripts/build.sh" ]; then
    "$PROJECT_ROOT/scripts/build.sh" check 2>&1 | tail -3
else
    cargo check -p clawft-kernel --features native 2>&1 | tail -3
fi

# 6. Show installed agents
echo ""
echo "--- Installed Agents ---"
for agent_dir in "$CLAUDE_AGENTS"/*/; do
    [ -d "$agent_dir" ] || continue
    agent_team=$(basename "$agent_dir")
    md_count=$(find "$agent_dir" -name "*.md" | wc -l)
    echo "  $agent_team ($md_count agent files)"
    # List individual agents
    for md in "$agent_dir"*.md; do
        [ -f "$md" ] || continue
        agent_name=$(basename "$md" .md)
        # Extract description from frontmatter if possible
        desc=$(grep -m1 '^description:' "$md" 2>/dev/null | sed 's/^description: *//' | head -c 60 || echo "")
        if [ -n "$desc" ]; then
            echo "    - $agent_name: $desc"
        else
            echo "    - $agent_name"
        fi
    done
done

# 7. Show installed skills
echo ""
echo "--- Installed Skills ---"
for skill_dir in "$CLAUDE_SKILLS"/*/; do
    [ -d "$skill_dir" ] || continue
    skill_name=$(basename "$skill_dir")
    md_count=$(find "$skill_dir" -name "*.md" | wc -l)
    echo "  $skill_name ($md_count skill files)"
done

# 8. Optional: initial ECC analysis
if [ "$ANALYZE" = "true" ]; then
    echo ""
    echo "--- Initial ECC Analysis ---"
    echo "  (Would run Weaver analysis — not yet implemented)"
    echo "  Future: weaver ecc analyze $PROJECT_ROOT --context 'clawft+WeftOS kernel'"
fi

echo ""
echo "=== weave init complete ==="
echo ""
echo "Available agents (invoke in Claude Code conversations):"
for agent_dir in "$CLAUDE_AGENTS"/*/; do
    [ -d "$agent_dir" ] || continue
    agent_team=$(basename "$agent_dir")
    for md in "$agent_dir"*.md; do
        [ -f "$md" ] && echo "  /agents/$agent_team/$(basename "$md" .md)"
    done
done
echo ""
echo "Available skills:"
for skill_dir in "$CLAUDE_SKILLS"/*/; do
    [ -d "$skill_dir" ] || continue
    skill_name=$(basename "$skill_dir")
    for md in "$skill_dir"*.md; do
        [ -f "$md" ] && echo "  /$skill_name/$(basename "$md" .md)"
    done
done
