# WeftOS Agent Team

Specialized agents for developing, operating, and evolving WeftOS.

## Team Topology

```
                    +-------------------+
                    | kernel-architect  |  Architecture & Phase Planning
                    +--------+----------+
                             |
          +------------------+------------------+
          |                  |                  |
+---------v---+    +---------v---+    +---------v---+
| Core Kernel |    | Network &   |    | Intelligence|
|             |    | Trust       |    | & Governance|
+-------------+    +-------------+    +-------------+
| process-    |    | mesh-       |    | weaver      |
|  supervisor |    |  engineer   |    | ecc-analyst |
| sandbox-    |    | chain-      |    | governance- |
|  warden     |    |  guardian   |    |  counsel    |
+------+------+    | defi-       |    +------+------+
       |           |  networker  |           |
       |           +------+------+           |
       |                  |                  |
       +------------------+------------------+
                          |
               +----------v----------+
               |     Operations      |
               +---------------------+
               | app-deployer        |
               | test-sentinel       |
               | doc-weaver          |
               +---------------------+
```

## Agent Directories

| Directory | Purpose |
|-----------|---------|
| `weftos/` | 12 specialized agents (this team) |
| `weftos-ecc/` | ECC Weaver skill (full specification) |
| `weftos-kernel/` | Kernel development skill |
| `weftos-mesh/` | Mesh networking skill |
| `clawft/` | General clawft development skill |

## Agents

### Core Kernel

| Agent | Type | Description |
|-------|------|-------------|
| **kernel-architect** | architect | Architecture and K0-K6 phase planning |
| **process-supervisor** | supervisor | Process lifecycle, restart strategies, health |
| **sandbox-warden** | security-specialist | WASM/container isolation, fuel metering |

### Networking & Trust

| Agent | Type | Description |
|-------|------|-------------|
| **mesh-engineer** | network-engineer | Mesh transport, discovery, 5-layer architecture |
| **chain-guardian** | security-engineer | ExoChain integrity, dual signing, RVF wire format |
| **defi-networker** | defi-specialist | Economic incentives, bond staking, trust progression |

### Intelligence & Governance

| Agent | Type | Description |
|-------|------|-------------|
| **weaver** | cognitive-modeler | ECC cognitive modeling, confidence-driven iteration |
| **ecc-analyst** | analyst | Causal analysis, spectral health, HNSW search |
| **governance-counsel** | governance-expert | Three-branch governance, effect vectors, trajectory learning |

### Operations

| Agent | Type | Description |
|-------|------|-------------|
| **app-deployer** | deployer | Application lifecycle, manifest parsing, rolling upgrades |
| **test-sentinel** | tester | Build gate, feature matrix testing, regression detection |
| **doc-weaver** | documenter | Fumadocs site, MDX content, SPARC plans |

## How It Works

1. Agents and skills are authored in this `agents/` directory (checked into git).
2. `scripts/weave-init.sh` copies them to `.claude/agents/` and `.claude/skills/` (gitignored).
3. Claude Code loads agents from `.claude/agents/` and skills from `.claude/skills/` at runtime.

## Installation

```bash
scripts/weave-init.sh --force
```

## Adding a New Agent

1. Create a new `.md` file in the appropriate `agents/` subdirectory
2. Include YAML frontmatter (name, type, description, capabilities, priority, hooks)
3. Write "You are a..." instructions with actual tool invocations and code patterns
4. Add an entry to the table above
5. Run `scripts/weave-init.sh --force` to install
