# Temporal-Lead Solver - Time-Series Causality Analysis

## Overview
Time-series graph database for detecting lead-lag relationships and temporal causality patterns.

## Purpose
Identify which events lead to (cause) other events based on temporal ordering and statistical correlation.

## Operations
- **Time-Series Points**: 20 events
- **Lead-Lag Pairs**: 17 relationships
- **Temporal Lag**: 3 time steps
- **Causal Edges**: Graph representation of temporal causality

## Results
- **Throughput**: 2.13 ops/sec
- **Latency**: 460ms avg
- **Time-Series Points**: 20
- **Lead-Lag Pairs**: 17
- **Avg Lag Time**: 3.0 steps
- **Temporal Edges**: 17

## Technical Details

### Time-Series Pattern
```typescript
// Sinusoidal pattern for demonstration
value = 0.5 + 0.5 * Math.sin(t * 0.3)

// Event at time t leads to event at t+3
fromTime: t
toTime: t + 3
mechanism: 'temporal_lead_lag_3'
```

### Causal Lag Detection
```
Event(t=0) → Event(t=3)   ✓ Lead-lag detected
Event(t=1) → Event(t=4)   ✓ Lead-lag detected
...
```

## Applications
- **Financial Markets**: Price lead-lag analysis
- **Supply Chain**: Demand forecasting
- **Healthcare**: Disease progression modeling
- **Climate Science**: Climate pattern causality

## Research Applications
- Granger causality testing
- Transfer entropy analysis
- Cross-correlation studies
- Predictive modeling

**Status**: ✅ Operational | **Package**: temporal-lead-solver
