Yes. Let’s stand up a minimal Claude Agent SDK service that runs multiple sub-agents in parallel inside Docker. It uses Promise.all to fan out tasks and then reconciles results.

# 1) Project layout

```
claude-agents-docker/
  Dockerfile
  docker-compose.yml           # optional
  package.json
  tsconfig.json
  src/
    index.ts
    agents/
      webResearchAgent.ts
      codeReviewAgent.ts
      dataAgent.ts
```

# 2) package.json

```json
{
  "name": "claude-agents-docker",
  "private": true,
  "type": "module",
  "scripts": {
    "start": "node --enable-source-maps dist/index.js",
    "build": "tsc -p .",
    "dev": "tsx src/index.ts"
  },
  "dependencies": {
    "@anthropic-ai/claude-agent-sdk": "^0.2.0",
    "@anthropic-ai/claude-code": "^2.1.0"
  },
  "devDependencies": {
    "tsx": "^4.19.0",
    "typescript": "^5.6.3"
  }
}
```

# 3) tsconfig.json

```json
{
  "compilerOptions": {
    "module": "esnext",
    "target": "es2022",
    "moduleResolution": "bundler",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

# 4) src/agents/*.ts

These are tiny task-specialized agents. You can swap in real tools later.

```ts
// src/agents/webResearchAgent.ts
import { createAgent } from "@anthropic-ai/claude-agent-sdk";

export function webResearchAgent() {
  return createAgent({
    name: "web-research",
    systemPrompt: `You perform fast web-style reconnaissance and return a concise bullet list of findings.`,
    tools: [] // add HTTP/search tools as you wire MCP or your own fetcher
  });
}

// src/agents/codeReviewAgent.ts
import { createAgent } from "@anthropic-ai/claude-agent-sdk";

export function codeReviewAgent() {
  return createAgent({
    name: "code-review",
    systemPrompt: `You review diffs and point out risks, complexity, and tests to add.`,
    tools: [] // later: git, fs, test runner hooks
  });
}

// src/agents/dataAgent.ts
import { createAgent } from "@anthropic-ai/claude-agent-sdk";

export function dataAgent() {
  return createAgent({
    name: "data-agent",
    systemPrompt: `You analyze tabular data and produce a short brief with 3 key stats and 1 risk.`,
    tools: [] // later: CSV/SQL connectors
  });
}
```

# 5) src/index.ts

Parallel fan-out with Promise.all, plus a simple reconcile step.

```ts
import "dotenv/config";
import { webResearchAgent } from "./agents/webResearchAgent.js";
import { codeReviewAgent } from "./agents/codeReviewAgent.js";
import { dataAgent } from "./agents/dataAgent.js";

async function main() {
  const topic = process.env.TOPIC ?? "migrate payments service";
  const codeDiff = process.env.DIFF ?? "feat: add payments router and mandate checks";
  const datasetHint = process.env.DATASET ?? "monthly tx volume, refunds, chargebacks";

  const research = webResearchAgent();
  const reviewer = codeReviewAgent();
  const analyst  = dataAgent();

  // Each agent runs with its own context. Fan out in parallel.
  const [researchOut, reviewOut, dataOut] = await Promise.all([
    research.run({ input: `Give me context and risks about: ${topic}` }),
    reviewer.run({ input: `Review this diff at a high level and propose tests:\n${codeDiff}` }),
    analyst.run({ input: `Analyze ${datasetHint} and report key stats.` })
  ]);

  // Basic reconcile step
  const summary = [
    "=== RESEARCH ===",
    researchOut.output?.trim() ?? "",
    "=== CODE REVIEW ===",
    reviewOut.output?.trim() ?? "",
    "=== DATA ===",
    dataOut.output?.trim() ?? ""
  ].join("\n");

  console.log(summary);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

# 6) Dockerfile

```dockerfile
FROM node:20-slim

# Avoid interactive tzdata prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install minimal deps if you later add git or build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY package.json package-lock.json* pnpm-lock.yaml* yarn.lock* ./
RUN npm ci --no-audit --no-fund

COPY tsconfig.json ./
COPY src ./src

RUN npm run build

# Runtime env
ENV NODE_ENV=production
# Required by Anthropic
ENV ANTHROPIC_API_KEY=""
# Optional task knobs
ENV TOPIC="payments mandates"
ENV DIFF="refactor: route validation"
ENV DATASET="transactions last 30 days"

CMD ["npm","start"]
```

# 7) docker-compose.yml (optional)

Scale out multiple identical workers for heavier fan-outs.

```yaml
services:
  agents:
    build: .
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      TOPIC: "upgrade checkout flow"
    deploy:
      replicas: 1
    restart: unless-stopped
```

# 8) Run it

```bash
# 1) set your key
export ANTHROPIC_API_KEY=sk-ant-...

# 2) build
docker build -t claude-agents:latest .

# 3) run
docker run --rm -e ANTHROPIC_API_KEY -e TOPIC="pricing page split" claude-agents:latest

# or with compose
docker compose up --build
```

# 9) Notes on real parallelism

* The Agent SDK exposes the same harness behind Claude Code and supports subagents plus background tasks. Your fan-out here is true process-level parallelism inside a single Node runtime using Promise.all. For more isolation, run N containers behind a queue. ([docs.claude.com][1])
* For heavy workflows, use an orchestrator pattern: a lead agent plans, spawns sub-agents in parallel, and aggregates results. This mirrors Anthropic’s multi-agent Research system. ([Anthropic][2])
* Sonnet 4.5 is positioned as their strongest coding and agenting model. Use it for better tool use and longer runs. ([Anthropic][3])

# 10) Where to extend

* Tools: add MCP tools or your HTTP/RPC clients to each agent.
* Memory: stitch in your Claude Flow memory hooks for pre/post tool use.
* Safety: isolate file writes per agent or route to separate branches.
* Scale: front the container with a queue. One “planner” job enqueues subtasks for pooled “worker” containers.

If you want, I can add MCP tools and a simple Redis queue so your planner container spawns dozens of sub-agents safely.

**References**
Anthropic Agent SDK overview. ([docs.claude.com][1])
Engineering note on subagents, hooks, and background tasks. ([Anthropic][4])
Anthropic multi-agent research system. ([Anthropic][2])
Sonnet 4.5 launch for complex agents. ([Anthropic][3])

[1]: https://docs.claude.com/en/api/agent-sdk/overview?utm_source=chatgpt.com "Agent SDK overview"
[2]: https://www.anthropic.com/engineering/built-multi-agent-research-system?utm_source=chatgpt.com "How we built our multi-agent research system"
[3]: https://www.anthropic.com/news/claude-sonnet-4-5?utm_source=chatgpt.com "Introducing Claude Sonnet 4.5"
[4]: https://anthropic.com/news/enabling-claude-code-to-work-more-autonomously?utm_source=chatgpt.com "Enabling Claude Code to work more autonomously"
