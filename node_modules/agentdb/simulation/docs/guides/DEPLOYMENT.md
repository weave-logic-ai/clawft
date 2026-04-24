# AgentDB v2.0 Production Deployment Guide

## Overview

This guide covers deploying AgentDB v2.0 in production environments, including system requirements, installation methods, configuration best practices, monitoring, and scaling considerations.

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation Methods](#installation-methods)
3. [Configuration](#configuration)
4. [Monitoring & Alerting](#monitoring--alerting)
5. [Scaling Strategies](#scaling-strategies)
6. [Security](#security)
7. [Backup & Recovery](#backup--recovery)
8. [Performance Tuning](#performance-tuning)

---

## System Requirements

### Minimum Requirements

**For Development/Testing**:
- **CPU**: 2 cores
- **RAM**: 4 GB
- **Disk**: 10 GB free space (SSD recommended)
- **Node.js**: 18.x or later
- **OS**: Linux, macOS, or Windows

**Network**:
- No external dependencies (fully embedded)
- Optional: Internet for npm package installation

### Recommended Requirements

**For Production**:
- **CPU**: 8 cores (16 threads)
- **RAM**: 16 GB (32 GB for large datasets)
- **Disk**: 50 GB SSD
- **Node.js**: 20.x LTS
- **OS**: Ubuntu 22.04 LTS or later

**Optional (for advanced features)**:
- **GPU**: NVIDIA GPU with CUDA 11.8+ (for neural augmentation)
- **PostgreSQL**: 15.x or later (for distributed deployments)

### Performance Scaling

| Dataset Size | Recommended RAM | Recommended CPU | Estimated Runtime |
|--------------|-----------------|-----------------|-------------------|
| < 100K vectors | 4 GB | 2 cores | < 5 minutes |
| 100K - 1M vectors | 8 GB | 4 cores | 5-30 minutes |
| 1M - 10M vectors | 16 GB | 8 cores | 30-120 minutes |
| > 10M vectors | 32+ GB | 16+ cores | 2+ hours |

---

## Installation Methods

### Method 1: npm (Development)

```bash
# Global installation
npm install -g agentdb@2.0

# Verify installation
agentdb --version
# Output: 2.0.0
```

**Pros**: Easy to install and update
**Cons**: Requires Node.js on target system

---

### Method 2: Docker (Production)

**Pull Official Image**:
```bash
docker pull agentdb/agentdb:2.0
```

**Run Simulation**:
```bash
docker run -v /data:/app/data agentdb/agentdb:2.0 simulate hnsw-exploration
```

**With Custom Configuration**:
```bash
docker run \
  -v /data:/app/data \
  -v /config/.agentdb.json:/app/.agentdb.json \
  agentdb/agentdb:2.0 \
  simulate hnsw-exploration
```

**Docker Compose**:
```yaml
# docker-compose.yml
version: '3.8'

services:
  agentdb:
    image: agentdb/agentdb:2.0
    volumes:
      - ./data:/app/data
      - ./config/.agentdb.json:/app/.agentdb.json
    environment:
      - AGENTDB_HNSW_M=32
      - AGENTDB_MEMORY_THRESHOLD=8192
    command: simulate hnsw-exploration
    deploy:
      resources:
        limits:
          cpus: '8'
          memory: 16G
```

**Build Custom Image**:
```dockerfile
# Dockerfile
FROM node:20-alpine

WORKDIR /app

# Install AgentDB
RUN npm install -g agentdb@2.0

# Copy configuration
COPY .agentdb.json .

# Expose metrics endpoint (optional)
EXPOSE 9090

ENTRYPOINT ["agentdb"]
CMD ["simulate", "hnsw-exploration"]
```

**Pros**: Isolated, reproducible, easy to scale
**Cons**: Slightly higher resource overhead

---

### Method 3: Standalone Binary (Air-gapped)

For environments without internet access or Node.js:

```bash
# Download binary
curl -O https://releases.agentdb.io/agentdb-linux-x64-2.0.0

# Make executable
chmod +x agentdb-linux-x64-2.0.0

# Run
./agentdb-linux-x64-2.0.0 simulate hnsw-exploration
```

**Available Binaries**:
- `agentdb-linux-x64-2.0.0` (Linux x86_64)
- `agentdb-macos-arm64-2.0.0` (macOS Apple Silicon)
- `agentdb-macos-x64-2.0.0` (macOS Intel)
- `agentdb-windows-x64-2.0.0.exe` (Windows)

**Pros**: No dependencies, works in air-gapped environments
**Cons**: Larger file size (~50MB), manual updates

---

### Method 4: Kubernetes (Distributed)

**Deployment**:
```yaml
# agentdb-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentdb
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agentdb
  template:
    metadata:
      labels:
        app: agentdb
    spec:
      containers:
      - name: agentdb
        image: agentdb/agentdb:2.0
        resources:
          requests:
            memory: "8Gi"
            cpu: "4"
          limits:
            memory: "16Gi"
            cpu: "8"
        volumeMounts:
        - name: config
          mountPath: /app/.agentdb.json
          subPath: .agentdb.json
        - name: data
          mountPath: /app/data
      volumes:
      - name: config
        configMap:
          name: agentdb-config
      - name: data
        persistentVolumeClaim:
          claimName: agentdb-data
```

**ConfigMap**:
```yaml
# agentdb-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: agentdb-config
data:
  .agentdb.json: |
    {
      "profile": "production",
      "hnsw": { "M": 32, "efConstruction": 200, "efSearch": 100 },
      "storage": { "reportPath": "/app/data/reports.db", "autoBackup": true },
      "monitoring": { "enabled": true, "alertThresholds": { "memoryMB": 12288, "latencyMs": 500 } }
    }
```

**PersistentVolumeClaim**:
```yaml
# agentdb-pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: agentdb-data
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 100Gi
  storageClassName: fast-ssd
```

**Pros**: Auto-scaling, high availability, orchestration
**Cons**: Complex setup, requires Kubernetes knowledge

---

## Configuration

### Production Configuration Best Practices

**File: `.agentdb.json`**

```json
{
  "profile": "production",

  "hnsw": {
    "M": 32,               // Optimal from simulations (8.2x speedup)
    "efConstruction": 200, // Balance quality vs. build time
    "efSearch": 100        // Balance recall vs. latency
  },

  "attention": {
    "heads": 8,            // Optimal from simulations (12.4% boost)
    "dimension": 64        // Standard dimension
  },

  "traversal": {
    "beamWidth": 5,        // Optimal (96.8% recall)
    "strategy": "beam"     // Best for production
  },

  "clustering": {
    "algorithm": "louvain", // Fast and effective (Q=0.758)
    "resolution": 1.0
  },

  "neural": {
    "mode": "full",             // Best accuracy (29.4% gain)
    "reinforcementLearning": true
  },

  "hypergraph": {
    "enabled": true,       // 3.7x speedup
    "maxEdgeSize": 10
  },

  "storage": {
    "reportPath": "/data/agentdb/reports.db",
    "autoBackup": true
  },

  "monitoring": {
    "enabled": true,
    "alertThresholds": {
      "memoryMB": 12288,   // Alert at 12GB
      "latencyMs": 500     // Alert at 500ms
    }
  },

  "logging": {
    "level": "info",
    "file": "/var/log/agentdb/simulation.log"
  }
}
```

### Environment Variable Overrides

For dynamic configuration:

```bash
# Override HNSW parameters
export AGENTDB_HNSW_M=64
export AGENTDB_HNSW_EF_CONSTRUCTION=400
export AGENTDB_HNSW_EF_SEARCH=200

# Override monitoring thresholds
export AGENTDB_MEMORY_THRESHOLD=16384
export AGENTDB_LATENCY_THRESHOLD=1000

# Override storage path
export AGENTDB_REPORT_PATH=/custom/path/reports.db

# Run simulation
agentdb simulate hnsw-exploration
```

---

## Monitoring & Alerting

### Built-in Health Monitoring

AgentDB v2.0 includes built-in health monitoring with self-healing (MPC algorithm):

```bash
# Enable monitoring
agentdb simulate hnsw-exploration --monitor
```

**Monitored Metrics**:
- CPU usage (%)
- Memory usage (MB)
- Heap usage (MB)
- Latency (ms)
- Throughput (ops/sec)
- Error rate

**Self-Healing Actions**:
- `pause_and_gc` - Force garbage collection
- `reduce_batch_size` - Throttle workload
- `restart_component` - Restart failed component
- `abort` - Abort on critical failure

---

### Prometheus Integration

**Expose Metrics Endpoint**:

```typescript
// Add to your application
import prometheus from 'prom-client';
import express from 'express';

const app = express();
const register = new prometheus.Registry();

// Define metrics
const simulationDuration = new prometheus.Histogram({
  name: 'agentdb_simulation_duration_seconds',
  help: 'Simulation execution time',
  labelNames: ['scenario']
});

const memoryUsage = new prometheus.Gauge({
  name: 'agentdb_memory_usage_bytes',
  help: 'Memory usage during simulation'
});

register.registerMetric(simulationDuration);
register.registerMetric(memoryUsage);

// Expose endpoint
app.get('/metrics', async (req, res) => {
  res.set('Content-Type', register.contentType);
  res.end(await register.metrics());
});

app.listen(9090);
```

**Prometheus Configuration**:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'agentdb'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

---

### Grafana Dashboard

**Sample Dashboard JSON**:

```json
{
  "dashboard": {
    "title": "AgentDB Monitoring",
    "panels": [
      {
        "title": "Memory Usage",
        "targets": [
          {
            "expr": "agentdb_memory_usage_bytes / 1024 / 1024"
          }
        ]
      },
      {
        "title": "Simulation Duration",
        "targets": [
          {
            "expr": "rate(agentdb_simulation_duration_seconds_sum[5m])"
          }
        ]
      }
    ]
  }
}
```

---

### Log Aggregation

**Filebeat Configuration**:

```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    paths:
      - /var/log/agentdb/*.log
    fields:
      app: agentdb
      environment: production

output.elasticsearch:
  hosts: ["localhost:9200"]
  index: "agentdb-%{+yyyy.MM.dd}"
```

**Logstash Pipeline**:

```
input {
  beats {
    port => 5044
  }
}

filter {
  if [app] == "agentdb" {
    grok {
      match => {
        "message" => "%{TIMESTAMP_ISO8601:timestamp} %{LOGLEVEL:level} %{GREEDYDATA:message}"
      }
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
  }
}
```

---

## Scaling Strategies

### Vertical Scaling

**Increase Resources**:
- CPU: 8 → 16 cores (2x faster multi-iteration)
- RAM: 16 → 32 GB (handle larger datasets)
- Disk: HDD → SSD (5-10x I/O speedup)

**Configuration Tuning**:
```json
{
  "hnsw": {
    "M": 64,              // Higher M for large datasets
    "efConstruction": 400
  },
  "neural": {
    "mode": "full"        // Enable all neural features
  }
}
```

---

### Horizontal Scaling (Distributed)

**Scenario Partitioning**:

Run different scenarios on different machines:

```bash
# Worker 1
agentdb simulate hnsw-exploration

# Worker 2
agentdb simulate attention-analysis

# Worker 3
agentdb simulate traversal-optimization
```

**Coordinator Script**:

```bash
#!/bin/bash
# coordinator.sh

WORKERS=("worker1:3000" "worker2:3000" "worker3:3000")
SCENARIOS=("hnsw-exploration" "attention-analysis" "traversal-optimization")

for i in "${!SCENARIOS[@]}"; do
  WORKER="${WORKERS[$i]}"
  SCENARIO="${SCENARIOS[$i]}"

  ssh "$WORKER" "agentdb simulate $SCENARIO" &
done

wait
```

---

### Auto-Scaling (Kubernetes)

**Horizontal Pod Autoscaler**:

```yaml
# agentdb-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agentdb-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: agentdb
  minReplicas: 3
  maxReplicas: 10
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

---

## Security

### File Permissions

```bash
# Restrict configuration file
chmod 600 .agentdb.json
chown agentdb:agentdb .agentdb.json

# Restrict database
chmod 600 /data/agentdb/reports.db
chown agentdb:agentdb /data/agentdb/reports.db
```

### Encryption at Rest

**SQLite Encryption**:

```bash
# Install SQLCipher
apt-get install sqlcipher

# Encrypt database
sqlcipher reports.db "PRAGMA key='your-encryption-key';"
```

**Configuration**:

```json
{
  "storage": {
    "reportPath": "/data/agentdb/reports.db",
    "encryption": {
      "enabled": true,
      "keyFile": "/secure/keyfile"
    }
  }
}
```

### Network Security

**Firewall Rules**:

```bash
# Allow only local connections to metrics endpoint
ufw allow from 127.0.0.1 to any port 9090

# Deny external access
ufw deny 9090
```

---

## Backup & Recovery

### Automated Backups

**Cron Job**:

```bash
# /etc/cron.d/agentdb-backup
0 2 * * * agentdb backup create /backups/agentdb-$(date +\%Y\%m\%d).db
```

**Backup Script**:

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backups"
DB_PATH="/data/agentdb/reports.db"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$BACKUP_DIR/agentdb-$TIMESTAMP.db"

# Create backup
agentdb backup create "$BACKUP_FILE"

# Compress
gzip "$BACKUP_FILE"

# Upload to S3 (optional)
aws s3 cp "$BACKUP_FILE.gz" s3://my-backups/agentdb/

# Rotate old backups (keep last 30 days)
find "$BACKUP_DIR" -name "agentdb-*.db.gz" -mtime +30 -delete
```

### Disaster Recovery

**Restore from Backup**:

```bash
# Stop AgentDB
systemctl stop agentdb

# Restore database
agentdb backup restore /backups/agentdb-20250130.db

# Verify
agentdb simulate --history | head -10

# Start AgentDB
systemctl start agentdb
```

---

## Performance Tuning

### Operating System Tuning

**Linux Kernel Parameters**:

```bash
# /etc/sysctl.conf

# Increase file descriptors
fs.file-max = 2097152

# Increase network buffers
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728

# Disable swap (for consistent performance)
vm.swappiness = 0

# Apply changes
sysctl -p
```

**CPU Affinity**:

```bash
# Pin AgentDB to specific CPU cores
taskset -c 0-7 agentdb simulate hnsw-exploration
```

---

### Node.js Tuning

**Increase Heap Size**:

```bash
export NODE_OPTIONS="--max-old-space-size=16384"  # 16GB
agentdb simulate hnsw-exploration
```

**Enable GC Exposure** (for self-healing):

```bash
node --expose-gc $(which agentdb) simulate hnsw-exploration
```

**Optimize V8**:

```bash
export NODE_OPTIONS="--max-old-space-size=16384 --optimize-for-size"
```

---

### Database Tuning

**SQLite Pragmas**:

```sql
-- Enable Write-Ahead Logging for better concurrency
PRAGMA journal_mode=WAL;

-- Increase cache size
PRAGMA cache_size=-2000000;  -- 2GB

-- Synchronous mode (balance safety vs. speed)
PRAGMA synchronous=NORMAL;

-- Memory-mapped I/O
PRAGMA mmap_size=268435456;  -- 256MB
```

---

## Troubleshooting

### Common Issues

**Issue: Out of Memory**

```bash
# Reduce memory footprint
agentdb simulate hnsw-exploration --profile memory

# Or adjust manually
export AGENTDB_MEMORY_THRESHOLD=4096
```

**Issue: Slow Performance**

```bash
# Use latency-optimized profile
agentdb simulate hnsw-exploration --profile latency

# Check system resources
top
iostat -x 1
```

**Issue: Database Locked**

```bash
# Check for hung processes
ps aux | grep agentdb

# Kill if necessary
pkill -9 agentdb

# Verify database integrity
sqlite3 reports.db "PRAGMA integrity_check;"
```

---

## Production Checklist

- [ ] Install AgentDB v2.0
- [ ] Configure `.agentdb.json` with production profile
- [ ] Set up monitoring (Prometheus + Grafana)
- [ ] Configure log aggregation (ELK/Loki)
- [ ] Set up automated backups (daily)
- [ ] Configure firewall rules
- [ ] Enable encryption at rest
- [ ] Tune OS and Node.js parameters
- [ ] Set up alerting (PagerDuty/Slack)
- [ ] Document runbooks for common issues
- [ ] Test disaster recovery plan

---

**Document Version**: 1.0
**Last Updated**: 2025-11-30
**Maintainer**: AgentDB Operations Team
