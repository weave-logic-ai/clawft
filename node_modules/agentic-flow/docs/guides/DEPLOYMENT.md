# 🚀 Deployment Options: Complete Guide

**5 deployment strategies • Local to cloud scale • Production-ready**

---

## 📑 Quick Navigation

[← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md) | [MCP Tools ←](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/MCP-TOOLS.md) | [Agent Booster →](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/AGENT-BOOSTER.md)

---

## 🎯 Deployment Strategy Overview

Choose the right deployment based on your needs:

| Deployment | Best For | Setup Time | Cost | Scale |
|------------|----------|------------|------|-------|
| **Local (npx)** | Development, testing | 30 seconds | Free | 1-10 agents |
| **Local (Global)** | Personal projects | 2 minutes | Free | 10-50 agents |
| **Docker** | CI/CD, production | 10 minutes | Low | 50-500 agents |
| **Kubernetes** | Enterprise scale | 30 minutes | Medium | 500-10K+ agents |
| **Flow Nexus Cloud** | Instant scale, managed | 5 minutes | Pay-as-you-go | Unlimited |

---

## 1️⃣ Local Development (npx)

**Perfect for**: Quick experiments, one-off tasks, development

### Quick Start

```bash
# No installation needed - run directly
npx agentic-flow --agent coder --task "Build a REST API"

# With specific model
npx agentic-flow \
  --agent researcher \
  --task "Analyze microservices trends" \
  --model "claude-3-5-sonnet-20241022"

# With streaming output
npx agentic-flow \
  --agent coder \
  --task "Create web scraper" \
  --stream
```

### Configuration

Set environment variables:

```bash
# API Keys
export ANTHROPIC_API_KEY=sk-ant-...
export OPENROUTER_API_KEY=sk-or-...
export GOOGLE_GEMINI_API_KEY=...

# Optional: Model preferences
export PROVIDER=anthropic  # anthropic, openrouter, gemini, onnx
export MODEL=claude-3-5-sonnet-20241022

# Optional: Cost optimization
export ROUTER_ENABLED=true
export ROUTER_PRIORITY=balanced  # cost, quality, speed, balanced
```

### Pros & Cons

**Pros:**
- ✅ Zero installation
- ✅ Perfect for quick tasks
- ✅ Always latest version
- ✅ No configuration needed

**Cons:**
- ❌ Slower startup (downloads on each run)
- ❌ No persistent configuration
- ❌ Limited to single agent tasks

---

## 2️⃣ Local Installation (Global)

**Perfect for**: Regular use, personal automation, local development

### Installation

```bash
# Install globally
npm install -g agentic-flow

# Verify installation
agentic-flow --version
agentic-flow --help
```

### Configuration

Create `~/.agentic-flow/config.json`:

```json
{
  "defaultProvider": "anthropic",
  "defaultModel": "claude-3-5-sonnet-20241022",
  "router": {
    "enabled": true,
    "priority": "balanced",
    "maxCost": 1.00
  },
  "providers": {
    "anthropic": {
      "apiKey": "${ANTHROPIC_API_KEY}",
      "baseUrl": "https://api.anthropic.com"
    },
    "openrouter": {
      "apiKey": "${OPENROUTER_API_KEY}",
      "baseUrl": "https://openrouter.ai/api/v1"
    },
    "gemini": {
      "apiKey": "${GOOGLE_GEMINI_API_KEY}"
    }
  },
  "reasoningbank": {
    "enabled": true,
    "dbPath": "~/.agentic-flow/memory.db"
  }
}
```

### Usage

```bash
# Run agents directly
agentic-flow --agent coder --task "Build API"

# List available agents
agentic-flow --list

# MCP server management
agentic-flow mcp add weather 'npx @modelcontextprotocol/server-weather'
agentic-flow mcp list
agentic-flow mcp remove weather

# ReasoningBank commands
agentic-flow reasoningbank init
agentic-flow reasoningbank status
```

### Advanced Configuration

Create `~/.agentic-flow/agents/custom-agent.md`:

```markdown
# Custom Developer Agent

## Role
Full-stack developer specializing in React and Node.js

## Capabilities
- Frontend: React, TypeScript, Tailwind
- Backend: Node.js, Express, PostgreSQL
- DevOps: Docker, GitHub Actions

## Instructions
You are an expert full-stack developer. Always:
1. Write TypeScript with strict types
2. Include comprehensive tests
3. Follow clean code principles
4. Document all public APIs
```

Then use it:

```bash
agentic-flow --agent custom-developer --task "Build dashboard"
```

### Pros & Cons

**Pros:**
- ✅ Fast startup (no downloads)
- ✅ Persistent configuration
- ✅ Custom agents support
- ✅ Full MCP tool access

**Cons:**
- ❌ Manual updates needed
- ❌ Single machine only
- ❌ Limited to local resources

---

## 3️⃣ Docker Deployment

**Perfect for**: CI/CD pipelines, reproducible environments, production

### Basic Dockerfile

Create `Dockerfile`:

```dockerfile
FROM node:18-alpine

# Install dependencies
RUN apk add --no-cache \
    git \
    python3 \
    make \
    g++

# Create app directory
WORKDIR /app

# Install agentic-flow
RUN npm install -g agentic-flow

# Copy configuration
COPY config.json /root/.agentic-flow/config.json

# Set environment variables
ENV ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
ENV OPENROUTER_API_KEY=${OPENROUTER_API_KEY}

# Entry point
ENTRYPOINT ["agentic-flow"]
CMD ["--help"]
```

### Build and Run

```bash
# Build image
docker build -t agentic-flow:latest .

# Run single agent
docker run --rm \
  -e ANTHROPIC_API_KEY=sk-ant-... \
  agentic-flow:latest \
  --agent coder \
  --task "Build REST API"

# Run with volume mount (for persistent memory)
docker run --rm \
  -e ANTHROPIC_API_KEY=sk-ant-... \
  -v $(pwd)/memory:/root/.agentic-flow \
  agentic-flow:latest \
  --agent researcher \
  --task "Analyze trends"
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  agent:
    build: .
    image: agentic-flow:latest
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
      - ROUTER_ENABLED=true
      - REASONINGBANK_ENABLED=true
    volumes:
      - ./config:/root/.agentic-flow
      - ./data:/data
    command: --agent coder --task "Build API"

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=agentic_flow
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"

volumes:
  postgres_data:
  redis_data:
```

Run with:

```bash
docker-compose up -d
docker-compose logs -f agent
docker-compose down
```

### CI/CD Integration

#### GitHub Actions

Create `.github/workflows/agent.yml`:

```yaml
name: Run Agentic Flow

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  code-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run code review agent
        uses: docker://agentic-flow:latest
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        with:
          args: --agent reviewer --task "Review PR changes"

      - name: Run tests
        run: npm test
```

#### GitLab CI

Create `.gitlab-ci.yml`:

```yaml
stages:
  - review
  - test

code-review:
  stage: review
  image: agentic-flow:latest
  script:
    - agentic-flow --agent reviewer --task "Review MR $CI_MERGE_REQUEST_IID"
  only:
    - merge_requests
  variables:
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
```

### Pros & Cons

**Pros:**
- ✅ Reproducible environments
- ✅ Easy CI/CD integration
- ✅ Version control for config
- ✅ Isolated dependencies

**Cons:**
- ❌ Docker overhead
- ❌ More complex setup
- ❌ Resource limits per container

---

## 4️⃣ Kubernetes Deployment

**Perfect for**: Enterprise scale, high availability, auto-scaling

### Prerequisites

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# Install Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### Kubernetes Manifests

#### 1. ConfigMap (`config.yaml`)

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: agentic-flow-config
  namespace: agentic-flow
data:
  config.json: |
    {
      "defaultProvider": "anthropic",
      "router": {
        "enabled": true,
        "priority": "balanced"
      }
    }
```

#### 2. Secret (`secrets.yaml`)

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: agentic-flow-secrets
  namespace: agentic-flow
type: Opaque
stringData:
  anthropic-api-key: sk-ant-...
  openrouter-api-key: sk-or-...
```

#### 3. Deployment (`deployment.yaml`)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentic-flow-agent
  namespace: agentic-flow
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agentic-flow
  template:
    metadata:
      labels:
        app: agentic-flow
    spec:
      containers:
      - name: agent
        image: agentic-flow:latest
        env:
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: agentic-flow-secrets
              key: anthropic-api-key
        - name: OPENROUTER_API_KEY
          valueFrom:
            secretKeyRef:
              name: agentic-flow-secrets
              key: openrouter-api-key
        - name: ROUTER_ENABLED
          value: "true"
        volumeMounts:
        - name: config
          mountPath: /root/.agentic-flow
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
      volumes:
      - name: config
        configMap:
          name: agentic-flow-config
```

#### 4. Service (`service.yaml`)

```yaml
apiVersion: v1
kind: Service
metadata:
  name: agentic-flow-service
  namespace: agentic-flow
spec:
  selector:
    app: agentic-flow
  ports:
  - protocol: TCP
    port: 8080
    targetPort: 8080
  type: LoadBalancer
```

#### 5. HorizontalPodAutoscaler (`hpa.yaml`)

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agentic-flow-hpa
  namespace: agentic-flow
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: agentic-flow-agent
  minReplicas: 3
  maxReplicas: 100
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Deploy to Kubernetes

```bash
# Create namespace
kubectl create namespace agentic-flow

# Apply manifests
kubectl apply -f config.yaml
kubectl apply -f secrets.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f hpa.yaml

# Verify deployment
kubectl get pods -n agentic-flow
kubectl get svc -n agentic-flow

# View logs
kubectl logs -f deployment/agentic-flow-agent -n agentic-flow

# Scale manually
kubectl scale deployment/agentic-flow-agent --replicas=10 -n agentic-flow
```

### Helm Chart

Create `helm/agentic-flow/values.yaml`:

```yaml
replicaCount: 3

image:
  repository: agentic-flow
  tag: latest
  pullPolicy: IfNotPresent

resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 100
  targetCPUUtilization: 70
  targetMemoryUtilization: 80

config:
  defaultProvider: anthropic
  router:
    enabled: true
    priority: balanced

secrets:
  anthropicApiKey: ""  # Set via --set secrets.anthropicApiKey=sk-ant-...
  openrouterApiKey: ""
```

Install with Helm:

```bash
helm install agentic-flow ./helm/agentic-flow \
  --namespace agentic-flow \
  --create-namespace \
  --set secrets.anthropicApiKey=sk-ant-... \
  --set secrets.openrouterApiKey=sk-or-...
```

### Pros & Cons

**Pros:**
- ✅ Auto-scaling (3-100+ pods)
- ✅ High availability
- ✅ Rolling updates
- ✅ Health checks & self-healing
- ✅ Enterprise-grade

**Cons:**
- ❌ Complex setup
- ❌ Higher infrastructure cost
- ❌ Requires Kubernetes expertise

---

## 5️⃣ Flow Nexus Cloud

**Perfect for**: Instant scale, managed infrastructure, zero DevOps

### Quick Start

```bash
# Install Flow Nexus CLI
npm install -g flow-nexus

# Register account
npx flow-nexus register

# Login
npx flow-nexus login

# Deploy swarm
npx flow-nexus swarm create \
  --topology mesh \
  --max-agents 10 \
  --strategy balanced
```

### Cloud Deployment via MCP

```javascript
// Initialize cloud swarm
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'swarm_init',
    params: {
      topology: 'mesh',
      maxAgents: 50,
      strategy: 'adaptive'
    }
  }
});

// Spawn cloud agents
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'agent_spawn',
    params: {
      type: 'coder',
      name: 'cloud-backend-dev'
    }
  }
});

// Create cloud sandbox for execution
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_create',
    params: {
      template: 'node',
      env_vars: {
        DATABASE_URL: '...',
        API_KEY: '...'
      }
    }
  }
});
```

### Features

- **Instant Scale**: 0 to 1000+ agents in seconds
- **Managed Infrastructure**: No servers to maintain
- **E2B Sandboxes**: Isolated execution environments
- **Neural Network Training**: Distributed GPU acceleration
- **Real-time Monitoring**: Live execution streams
- **Pay-as-you-go**: Only pay for what you use

### Pricing

| Tier | Price | Agents | Features |
|------|-------|--------|----------|
| **Free** | $0/month | 5 concurrent | Basic features |
| **Pro** | $29/month | 50 concurrent | Neural networks, sandboxes |
| **Team** | $99/month | 200 concurrent | Priority support, SLA |
| **Enterprise** | Custom | Unlimited | Dedicated, custom SLA |

### Pros & Cons

**Pros:**
- ✅ Zero infrastructure management
- ✅ Instant global scale
- ✅ Built-in neural networks
- ✅ E2B sandbox integration
- ✅ Real-time monitoring

**Cons:**
- ❌ Requires internet connection
- ❌ Monthly cost (after free tier)
- ❌ Data leaves your infrastructure

---

## 🔒 Security Best Practices

### API Key Management

```bash
# Use environment variables
export ANTHROPIC_API_KEY=sk-ant-...

# Use secrets management (Kubernetes)
kubectl create secret generic api-keys \
  --from-literal=anthropic=sk-ant-... \
  --from-literal=openrouter=sk-or-...

# Use HashiCorp Vault
vault kv put secret/agentic-flow \
  anthropic_key=sk-ant-... \
  openrouter_key=sk-or-...
```

### Network Security

```yaml
# Kubernetes NetworkPolicy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: agentic-flow-netpol
  namespace: agentic-flow
spec:
  podSelector:
    matchLabels:
      app: agentic-flow
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: agentic-flow
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - namespaceSelector: {}
    ports:
    - protocol: TCP
      port: 443  # HTTPS only
```

### Data Protection

- **Encrypt at rest**: Use encrypted volumes
- **Encrypt in transit**: TLS 1.3 for all communications
- **PII scrubbing**: Enable ReasoningBank PII scrubber
- **Audit logging**: Track all API calls and agent actions

---

## 📊 Performance Optimization

### Local Optimization

```bash
# Enable caching
export CACHE_ENABLED=true
export CACHE_TTL=3600

# Limit concurrency
export MAX_CONCURRENT_AGENTS=5

# Use local ONNX models
export PROVIDER=onnx
export ONNX_MODEL_PATH=./models/phi-4
```

### Docker Optimization

```dockerfile
# Multi-stage build for smaller images
FROM node:18-alpine AS builder
WORKDIR /app
RUN npm install -g agentic-flow --production

FROM node:18-alpine
COPY --from=builder /usr/local/lib/node_modules /usr/local/lib/node_modules
COPY --from=builder /usr/local/bin/agentic-flow /usr/local/bin/agentic-flow
```

### Kubernetes Optimization

```yaml
# Resource limits
resources:
  requests:
    memory: "256Mi"  # Start small
    cpu: "250m"
  limits:
    memory: "1Gi"    # Allow bursts
    cpu: "1000m"

# Pod affinity for better locality
affinity:
  podAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
    - weight: 100
      podAffinityTerm:
        labelSelector:
          matchLabels:
            app: agentic-flow
        topologyKey: kubernetes.io/hostname
```

---

## 📈 Monitoring & Observability

### Prometheus Metrics

```yaml
# ServiceMonitor for Prometheus
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: agentic-flow
  namespace: agentic-flow
spec:
  selector:
    matchLabels:
      app: agentic-flow
  endpoints:
  - port: metrics
    interval: 30s
```

### Grafana Dashboard

Import dashboard JSON:
- Panel 1: Agent count over time
- Panel 2: Task success rate
- Panel 3: Average task duration
- Panel 4: Token usage
- Panel 5: Cost tracking

### Logging

```yaml
# Fluentd configuration
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/agentic-flow/*.log
      tag agentic-flow.*
    </source>
    <match agentic-flow.**>
      @type elasticsearch
      host elasticsearch.logging.svc
      port 9200
    </match>
```

---

## 🔗 Related Documentation

### Core Components
- [← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md)
- [Agent Booster (Code Transformations)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/AGENT-BOOSTER.md)
- [ReasoningBank (Learning Memory)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/REASONINGBANK.md)
- [Multi-Model Router (Cost Optimization)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/MULTI-MODEL-ROUTER.md)
- [MCP Tools Reference](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/MCP-TOOLS.md)

### External Resources
- [Docker Documentation](https://docs.docker.com)
- [Kubernetes Documentation](https://kubernetes.io/docs)
- [Flow Nexus Platform](https://flow-nexus.ruv.io)
- [E2B Sandboxes](https://e2b.dev)

---

## 🤝 Contributing

Deployment improvements welcome! See [CONTRIBUTING.md](https://github.com/ruvnet/agentic-flow/blob/main/CONTRIBUTING.md).

---

## 📄 License

MIT License - see [LICENSE](https://github.com/ruvnet/agentic-flow/blob/main/LICENSE) for details.

---

**Deploy anywhere. Local to cloud. Production-ready.** 🚀

[← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md)
