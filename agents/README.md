# clawft+WeftOS Agent Skills

Source-of-truth directory for agent/skill definitions. These are copied to
`.claude/skills/` at runtime by `scripts/weave-init.sh` (or `weaver init`).

## Available Skills

| Skill | Directory | Description |
|-------|-----------|-------------|
| WEAVER | `weftos-ecc/WEAVER.md` | Self-evolving cognitive modeler -- ECC kernel-native SystemService for causal model discovery |
| KERNEL | `weftos-kernel/KERNEL.md` | WeftOS kernel development -- adding modules, feature flags, testing patterns, K0-K6 phases |
| MESH | `weftos-mesh/MESH.md` | Mesh networking development -- 5-layer architecture, transport traits, frame types, discovery |
| CLAWFT | `clawft/CLAWFT.md` | General clawft development -- crate structure, plugin system, providers, channels, tools |

## How It Works

1. Skills are authored and maintained in this `agents/` directory (checked into git).
2. `scripts/weave-init.sh` (or `weaver init`) copies them to `.claude/skills/` (gitignored).
3. Claude Code loads skills from `.claude/skills/` at runtime.

This follows the ruflo pattern: the repo is the source of truth, `.claude/` is the runtime copy.

## Adding a New Skill

1. Create a new directory under `agents/` (e.g., `agents/my-skill/`)
2. Add a markdown file with YAML frontmatter (name, description, version, category, tags)
3. Add an entry to the table above
4. Run `scripts/weave-init.sh --force` to install
