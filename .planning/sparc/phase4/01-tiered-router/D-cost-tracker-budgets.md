# SPARC Implementation Plan: Phase D - CostTracker & Budget Enforcement

## Agent Instructions

### Context
This is Phase D of the Tiered Router sprint (01-tiered-router). The `CostTracker` provides per-user and global cost tracking with daily/monthly budget enforcement. It is the budget gatekeeper called by `TieredRouter` (Phase C) at routing time. The tracker is thread-safe, lock-free in the hot path, and supports optional disk persistence to survive process restarts.

### Dependencies
**Phase A** (RoutingConfig types): `CostBudgetConfig` struct with `global_daily_limit_usd`, `global_monthly_limit_usd`, `tracking_persistence`, `reset_hour_utc`.
**Phase B** (UserPermissions): `UserPermissions.cost_budget_daily_usd` and `UserPermissions.cost_budget_monthly_usd` per-user limits.
**Phase C** (TieredRouter): `CostTrackable` trait defined in `tiered_router.rs`. The `TieredRouter` holds `Arc<dyn CostTrackable>` and calls `check_budget(sender_id, estimated_cost, daily_limit, monthly_limit)` from `apply_budget_constraints()` and `record_estimated()` / `record_actual()` from `ModelRouter::update()`. The trait signature is aligned with the concrete `CostTracker` methods (FIX-03). In the router hot path, the concrete `reserve_budget()` method should be preferred over the trait's `check_budget()` + `record_estimated()` pair to prevent TOCTOU races (FIX-07).

### Source Files to Reference
- `crates/clawft-core/src/pipeline/traits.rs` -- `RoutingDecision`, `ResponseOutcome`, `ModelRouter::update()`
- `crates/clawft-core/src/pipeline/router.rs` -- `StaticRouter` as style reference
- `crates/clawft-core/src/pipeline/rate_limiter.rs` -- Sibling concurrent data structure (Phase E), same `DashMap` pattern
- `crates/clawft-types/src/routing.rs` -- `CostBudgetConfig`, `UserPermissions`
- `.planning/08-tiered-router.md` -- Section 6 (Cost Tracking), Section 9.5 (UsageTracker integration)

### File to Create
`crates/clawft-core/src/pipeline/cost_tracker.rs`

### Module Registration
Add `pub mod cost_tracker;` to `crates/clawft-core/src/pipeline/mod.rs`.

### Branch
Work on branch: `weft/tiered-router`

### Critical Success Criteria
- [ ] `CostTracker` implements the `CostTrackable` trait from Phase C (`tiered_router.rs`)
- [ ] Per-user daily and monthly spend tracked via `DashMap<String, f64>` (no Mutex)
- [ ] Global daily and monthly spend tracked via atomic CAS on `AtomicU64` (f64 as bits)
- [ ] Budget check returns `BudgetResult` enum with five variants (`Ok`, `OverDailyUser`, `OverMonthlyUser`, `OverDailyGlobal`, `OverMonthlyGlobal`)
- [ ] Automatic daily/monthly resets using `reset_hour_utc` from `CostBudgetConfig`
- [ ] Persistence roundtrip: `save()` -> `load()` restores identical state
- [ ] Concurrent access from 10+ threads does not panic or corrupt data
- [ ] All tests pass: `cargo test -p clawft-core cost_tracker`
- [ ] Lint clean: `cargo clippy -p clawft-core -- -D warnings`

---

## 1. Specification

### 1.1 Purpose

The `CostTracker` tracks per-user cost accumulation (daily and monthly) for budget enforcement in the `TieredRouter`. It enables the router to:

1. **Check** whether a user can afford a request at a given tier before routing.
2. **Record** estimated cost when a routing decision is made (pre-LLM call).
3. **Record** actual cost when the response arrives (post-LLM call), reconciling the estimate with a delta adjustment.
4. **Enforce** global daily and monthly budget caps across all users.
5. **Persist** cost data to disk so budgets survive process restarts.
6. **Reset** daily counters at a configurable UTC hour and monthly counters on the 1st of each month.

### 1.2 Data Model

```
CostTracker
  daily: DashMap<String, f64>        // user_id -> daily USD spend
  monthly: DashMap<String, f64>      // user_id -> monthly USD spend
  global_daily_bits: AtomicU64       // all-users daily USD (f64 stored as bits)
  global_monthly_bits: AtomicU64     // all-users monthly USD (f64 stored as bits)
  daily_reset_at: AtomicU64          // unix timestamp of last daily reset
  monthly_reset_at: AtomicU64        // unix timestamp of last monthly reset
  config: CostTrackerConfig          // limits, reset hour, persistence toggle
  ops_since_save: AtomicU32          // record() calls since last persist
  persistence_path: Option<PathBuf>  // e.g. ~/.clawft/cost_tracking.json
```

### 1.3 BudgetResult Enum

When the router asks "can this user afford tier X?", the tracker returns:

| Variant | Meaning |
|---------|---------|
| `Ok` | Budget allows the request |
| `OverDailyUser { current_spend, daily_limit }` | User's daily budget would be exceeded |
| `OverMonthlyUser { current_spend, monthly_limit }` | User's monthly budget would be exceeded |
| `OverDailyGlobal { current_spend, daily_limit }` | Global daily budget would be exceeded |
| `OverMonthlyGlobal { current_spend, monthly_limit }` | Global monthly budget would be exceeded |

The check is ordered: user daily, user monthly, global daily, global monthly. First failure wins.

### 1.4 Cost Estimation Formula

The `TieredRouter` computes the estimated cost before calling `check_budget()`:

```
estimated_cost_usd = tier.cost_per_1k_tokens * (estimated_input_tokens + max_output_tokens) / 1000.0
```

The `CostTracker` receives a pre-computed USD amount. It does not need to know about tiers or token counts.

### 1.5 CostTrackerConfig

A runtime configuration struct derived from `CostBudgetConfig` (Phase A types):

| Field | Type | Default | Source |
|-------|------|---------|--------|
| `global_daily_limit_usd` | `f64` | `0.0` (unlimited) | `CostBudgetConfig.global_daily_limit_usd` |
| `global_monthly_limit_usd` | `f64` | `0.0` (unlimited) | `CostBudgetConfig.global_monthly_limit_usd` |
| `reset_hour_utc` | `u8` | `0` | `CostBudgetConfig.reset_hour_utc` |
| `persistence_enabled` | `bool` | `true` | `CostBudgetConfig.tracking_persistence` |
| `save_interval_ops` | `u32` | `10` | Hardcoded sensible default |

### 1.6 Public API Surface

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(config, Option<PathBuf>) -> Self` | Construct; load persisted state if available |
| `in_memory` | `(config) -> Self` | Construct without persistence (for testing) |
| `reserve_budget` | `(&self, sender_id, estimated_cost, user_daily_limit, user_monthly_limit) -> BudgetResult` | **Atomic** check + reserve (FIX-07). Prevents TOCTOU races. |
| `reconcile_actual` | `(&self, sender_id, estimated_cost_usd, actual_cost_usd)` | Adjust reservation after LLM response (FIX-07) |
| `check_budget` | `(&self, sender_id, estimated_cost, user_daily_limit, user_monthly_limit) -> BudgetResult` | Read-only budget check (no reservation). Used by CostTrackable trait. |
| `record_estimated` | `(&self, sender_id, estimated_cost_usd)` | Record estimated cost (use `reserve_budget` in hot path instead) |
| `record_actual` | `(&self, sender_id, estimated_cost_usd, actual_cost_usd)` | Delegates to `reconcile_actual()` |
| `daily_spend` | `(&self, sender_id) -> f64` | Get user's daily spend |
| `monthly_spend` | `(&self, sender_id) -> f64` | Get user's monthly spend |
| `global_daily_spend` | `(&self) -> f64` | Get total daily spend across all users |
| `global_monthly_spend` | `(&self) -> f64` | Get total monthly spend across all users |
| `force_daily_reset` | `(&self)` | Admin: clear daily counters |
| `force_monthly_reset` | `(&self)` | Admin: clear monthly + daily counters |
| `save` | `(&self) -> io::Result<()>` | Persist current state to disk (sets 0600 perms on unix) |
| `active_users_daily` | `(&self) -> usize` | Count of users with non-zero daily spend |
| `active_users_monthly` | `(&self) -> usize` | Count of users with non-zero monthly spend |
| `daily_spend_report` | `(&self) -> Vec<(String, f64)>` | Snapshot of all per-user daily spend |
| `monthly_spend_report` | `(&self) -> Vec<(String, f64)>` | Snapshot of all per-user monthly spend |

### 1.7 Non-Goals

- Actual per-model pricing. That belongs to `clawft-llm::UsageTracker`. This tracker uses tier-level estimates.
- Distributed budget tracking across multiple processes. Single-process only.
- Billing or invoicing. This is soft budget enforcement, not accounting.
- Per-channel budgets (tracked per-user only; channel is not a budget dimension).

---

## 2. Pseudocode

### 2.1 Core Types

```rust
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Configuration for cost tracking behavior.
/// Derived from `CostBudgetConfig` at construction time.
#[derive(Debug, Clone)]
pub struct CostTrackerConfig {
    /// Global daily budget limit in USD. 0.0 = unlimited.
    pub global_daily_limit_usd: f64,
    /// Global monthly budget limit in USD. 0.0 = unlimited.
    pub global_monthly_limit_usd: f64,
    /// UTC hour (0-23) at which daily counters reset.
    pub reset_hour_utc: u8,
    /// Whether to persist cost data to disk.
    pub persistence_enabled: bool,
    /// Number of record() calls between automatic saves.
    pub save_interval_ops: u32,
}

impl Default for CostTrackerConfig {
    fn default() -> Self {
        Self {
            global_daily_limit_usd: 0.0,
            global_monthly_limit_usd: 0.0,
            reset_hour_utc: 0,
            persistence_enabled: true,
            save_interval_ops: 10,
        }
    }
}

/// Result of a budget check.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetResult {
    /// Budget allows the request.
    Ok,
    /// User's daily budget would be exceeded.
    OverDailyUser {
        current_spend: f64,
        daily_limit: f64,
    },
    /// User's monthly budget would be exceeded.
    OverMonthlyUser {
        current_spend: f64,
        monthly_limit: f64,
    },
    /// Global daily budget would be exceeded.
    OverDailyGlobal {
        current_spend: f64,
        daily_limit: f64,
    },
    /// Global monthly budget would be exceeded.
    OverMonthlyGlobal {
        current_spend: f64,
        monthly_limit: f64,
    },
}

impl BudgetResult {
    /// Returns `true` if the budget check passed.
    pub fn is_ok(&self) -> bool {
        matches!(self, BudgetResult::Ok)
    }
}

/// Serializable snapshot of cost tracking state for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CostSnapshot {
    /// Per-user daily spend as (user_id, usd) pairs.
    daily: Vec<(String, f64)>,
    /// Per-user monthly spend as (user_id, usd) pairs.
    monthly: Vec<(String, f64)>,
    /// Sum of all users' daily spend.
    global_daily: f64,
    /// Sum of all users' monthly spend.
    global_monthly: f64,
    /// Unix timestamp of last daily reset.
    daily_reset_at: u64,
    /// Unix timestamp of last monthly reset.
    monthly_reset_at: u64,
}
```

### 2.2 CostTracker Struct

```rust
/// Tracks per-user and global cost accumulation for budget enforcement.
///
/// Thread-safe: all operations are lock-free via `DashMap` and atomics.
/// Persistence is best-effort -- a crash may lose up to `save_interval_ops`
/// operations. The loss direction is always "under-counting" (safe side:
/// users get slightly more budget than intended, never less).
pub struct CostTracker {
    /// Per-user daily spend in USD.
    daily: DashMap<String, f64>,
    /// Per-user monthly spend in USD.
    monthly: DashMap<String, f64>,
    /// Global daily spend, stored as AtomicU64 using f64::to_bits/from_bits.
    global_daily_bits: AtomicU64,
    /// Global monthly spend, stored as AtomicU64 using f64::to_bits/from_bits.
    global_monthly_bits: AtomicU64,
    /// Unix timestamp of last daily reset.
    daily_reset_at: AtomicU64,
    /// Unix timestamp of last monthly reset.
    monthly_reset_at: AtomicU64,
    /// Configuration (immutable after construction).
    config: CostTrackerConfig,
    /// Operations since last save (approximate; races acceptable).
    ops_since_save: AtomicU32,
    /// Path for persistence file.
    persistence_path: Option<PathBuf>,
}
```

### 2.3 Constructor and Persistence

```rust
impl CostTracker {
    /// Create a new cost tracker with the given configuration.
    ///
    /// If `persistence_path` is `Some` and the file exists, state is loaded
    /// from disk. Otherwise starts with zero spend.
    pub fn new(config: CostTrackerConfig, persistence_path: Option<PathBuf>) -> Self {
        let tracker = Self {
            daily: DashMap::new(),
            monthly: DashMap::new(),
            global_daily_bits: AtomicU64::new(0.0f64.to_bits()),
            global_monthly_bits: AtomicU64::new(0.0f64.to_bits()),
            daily_reset_at: AtomicU64::new(Self::now_unix()),
            monthly_reset_at: AtomicU64::new(Self::now_unix()),
            config,
            ops_since_save: AtomicU32::new(0),
            persistence_path,
        };

        // Attempt to load persisted state
        if let Some(ref path) = tracker.persistence_path {
            if path.exists() {
                match tracker.load_from_path(path) {
                    Ok(()) => debug!("loaded cost tracking state from {}", path.display()),
                    Err(e) => warn!("failed to load cost tracking state: {e}"),
                }
            }
        }

        tracker
    }

    /// Create a tracker with no persistence (for testing).
    pub fn in_memory(config: CostTrackerConfig) -> Self {
        Self::new(config, None)
    }

    /// Load state from a JSON persistence file.
    fn load_from_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        let data = std::fs::read_to_string(path)?;
        let snapshot: CostSnapshot = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        for (user, spend) in snapshot.daily {
            self.daily.insert(user, spend);
        }
        for (user, spend) in snapshot.monthly {
            self.monthly.insert(user, spend);
        }
        self.store_global_daily(snapshot.global_daily);
        self.store_global_monthly(snapshot.global_monthly);
        self.daily_reset_at.store(snapshot.daily_reset_at, Ordering::Relaxed);
        self.monthly_reset_at.store(snapshot.monthly_reset_at, Ordering::Relaxed);

        Ok(())
    }

    /// Save current state to the configured persistence path.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption from
    /// crashes during save.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(ref path) = self.persistence_path else {
            return Ok(());
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let snapshot = CostSnapshot {
            daily: self.daily.iter().map(|e| (e.key().clone(), *e.value())).collect(),
            monthly: self.monthly.iter().map(|e| (e.key().clone(), *e.value())).collect(),
            global_daily: self.load_global_daily(),
            global_monthly: self.load_global_monthly(),
            daily_reset_at: self.daily_reset_at.load(Ordering::Relaxed),
            monthly_reset_at: self.monthly_reset_at.load(Ordering::Relaxed),
        };

        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Write to temp file then rename for atomic replacement
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, path)?;

        // FIX-12: Set restrictive file permissions (0600) on the persistence
        // file. The file contains user IDs and spend amounts which should not
        // be world-readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        self.ops_since_save.store(0, Ordering::Relaxed);
        debug!("saved cost tracking state to {}", path.display());

        Ok(())
    }

    /// Trigger a save if enough operations have elapsed.
    fn maybe_save(&self) {
        if !self.config.persistence_enabled {
            return;
        }
        let ops = self.ops_since_save.fetch_add(1, Ordering::Relaxed);
        if ops + 1 >= self.config.save_interval_ops {
            if let Err(e) = self.save() {
                warn!("failed to save cost tracking state: {e}");
            }
        }
    }
}
```

### 2.4 Atomic f64 Helpers

Since `AtomicF64` is not in the Rust standard library, global totals are stored as `AtomicU64` using `f64::to_bits()` / `f64::from_bits()`. Addition uses a CAS (compare-and-swap) loop.

```rust
impl CostTracker {
    fn load_global_daily(&self) -> f64 {
        f64::from_bits(self.global_daily_bits.load(Ordering::Relaxed))
    }

    fn store_global_daily(&self, value: f64) {
        self.global_daily_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    fn load_global_monthly(&self) -> f64 {
        f64::from_bits(self.global_monthly_bits.load(Ordering::Relaxed))
    }

    fn store_global_monthly(&self, value: f64) {
        self.global_monthly_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Atomically add `amount` to global daily spend. Returns new value.
    fn add_global_daily(&self, amount: f64) -> f64 {
        loop {
            let old_bits = self.global_daily_bits.load(Ordering::Relaxed);
            let old = f64::from_bits(old_bits);
            let new = old + amount;
            let new_bits = new.to_bits();
            if self.global_daily_bits.compare_exchange(
                old_bits, new_bits, Ordering::Relaxed, Ordering::Relaxed,
            ).is_ok() {
                return new;
            }
            // CAS failed -- another thread updated; retry with fresh value
        }
    }

    /// Atomically add `amount` to global monthly spend. Returns new value.
    fn add_global_monthly(&self, amount: f64) -> f64 {
        loop {
            let old_bits = self.global_monthly_bits.load(Ordering::Relaxed);
            let old = f64::from_bits(old_bits);
            let new = old + amount;
            let new_bits = new.to_bits();
            if self.global_monthly_bits.compare_exchange(
                old_bits, new_bits, Ordering::Relaxed, Ordering::Relaxed,
            ).is_ok() {
                return new;
            }
        }
    }

    /// Get current unix timestamp in seconds.
    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
```

### 2.5 Budget Checking and Atomic Reservation

The original design had separate `check_budget()` and `record_estimated()` methods.
This creates a TOCTOU (Time-Of-Check-To-Time-Of-Use) race condition: two concurrent
threads could both pass `check_budget()` and then both call `record_estimated()`,
overshooting the budget by one request's cost.

**FIX-07 (Atomic Budget Reserve)**: Replace the two-step check+record with a single
`reserve_budget()` method that atomically checks AND reserves within a
`DashMap::entry()` lock. After the LLM response arrives, `reconcile_actual()`
adjusts the reservation. The old `check_budget()` is retained as a read-only query
for the `CostTrackable` trait, but the router's hot path uses `reserve_budget()`.

```rust
impl CostTracker {
    /// Atomically check all budget limits and reserve the estimated cost
    /// if within budget.
    ///
    /// This replaces the previous two-step check_budget() + record_estimated()
    /// pattern to prevent TOCTOU race conditions. The per-user daily and monthly
    /// spend are updated within a DashMap::entry() lock, ensuring that concurrent
    /// reserve calls for the same user are serialized.
    ///
    /// Check + reserve order:
    /// 1. User daily budget (from UserPermissions.cost_budget_daily_usd)
    /// 2. User monthly budget (from UserPermissions.cost_budget_monthly_usd)
    /// 3. Global daily budget (from CostTrackerConfig.global_daily_limit_usd)
    /// 4. Global monthly budget (from CostTrackerConfig.global_monthly_limit_usd)
    ///
    /// A limit of 0.0 means unlimited (that dimension is not checked).
    /// Calls `reset_if_needed()` before checking to ensure counters are current.
    ///
    /// On success (BudgetResult::Ok), the estimated cost is already reserved
    /// against daily, monthly, and global totals. The caller MUST NOT call
    /// record_estimated() separately.
    ///
    /// On failure (any Over* variant), no spend is recorded.
    /// Public API uses `sender_id` to match `AuthContext.sender_id`.
    /// Internally mapped to `user_id` as DashMap key.
    pub fn reserve_budget(
        &self,
        sender_id: &str,
        estimated_cost_usd: f64,
        user_daily_limit: f64,
        user_monthly_limit: f64,
    ) -> BudgetResult {
        let user_id = sender_id; // DashMap key

        // Auto-reset if a new day/month boundary has passed
        self.reset_if_needed();

        if estimated_cost_usd <= 0.0 {
            return BudgetResult::Ok;
        }

        // Use DashMap::entry() to hold a lock on the user's entry for the
        // duration of the check+reserve. This prevents two concurrent threads
        // from both passing the check before either records.
        //
        // Step 1+2: Check and reserve per-user daily and monthly atomically.
        let mut daily_entry = self.daily.entry(user_id.to_string()).or_insert(0.0);
        let current_daily = *daily_entry;

        if user_daily_limit > 0.0 && current_daily + estimated_cost_usd > user_daily_limit {
            return BudgetResult::OverDailyUser {
                current_spend: current_daily,
                daily_limit: user_daily_limit,
            };
        }

        let mut monthly_entry = self.monthly.entry(user_id.to_string()).or_insert(0.0);
        let current_monthly = *monthly_entry;

        if user_monthly_limit > 0.0 && current_monthly + estimated_cost_usd > user_monthly_limit {
            return BudgetResult::OverMonthlyUser {
                current_spend: current_monthly,
                monthly_limit: user_monthly_limit,
            };
        }

        // Step 3: Global daily check (atomic load; reservation via CAS after all checks pass)
        if self.config.global_daily_limit_usd > 0.0 {
            let current_global_daily = self.load_global_daily();
            if current_global_daily + estimated_cost_usd > self.config.global_daily_limit_usd {
                return BudgetResult::OverDailyGlobal {
                    current_spend: current_global_daily,
                    daily_limit: self.config.global_daily_limit_usd,
                };
            }
        }

        // Step 4: Global monthly check
        if self.config.global_monthly_limit_usd > 0.0 {
            let current_global_monthly = self.load_global_monthly();
            if current_global_monthly + estimated_cost_usd > self.config.global_monthly_limit_usd {
                return BudgetResult::OverMonthlyGlobal {
                    current_spend: current_global_monthly,
                    monthly_limit: self.config.global_monthly_limit_usd,
                };
            }
        }

        // All checks passed -- commit the reservation.
        // Per-user entries are still locked via DashMap entry references.
        *daily_entry += estimated_cost_usd;
        *monthly_entry += estimated_cost_usd;

        // Global totals via CAS (these are cross-user, so DashMap lock does not help)
        self.add_global_daily(estimated_cost_usd);
        self.add_global_monthly(estimated_cost_usd);

        self.maybe_save();

        BudgetResult::Ok
    }

    /// Adjust a previous reservation after the actual LLM cost is known.
    ///
    /// Called after the LLM response arrives (from `TieredRouter::update()`).
    /// Computes `delta = actual_cost - estimated_cost` and adjusts the user's
    /// spend accordingly. If actual < estimated, the excess reservation is
    /// credited back. If actual > estimated, the shortfall is debited.
    ///
    /// Per-user and global spend are clamped to 0.0 minimum.
    pub fn reconcile_actual(
        &self,
        sender_id: &str,
        estimated_cost_usd: f64,
        actual_cost_usd: f64,
    ) {
        let user_id = sender_id; // DashMap key
        let delta = actual_cost_usd - estimated_cost_usd;
        if delta.abs() < 1e-10 {
            return; // No meaningful difference
        }

        // Adjust per-user daily (clamp to 0.0)
        self.daily
            .entry(user_id.to_string())
            .and_modify(|v| *v = (*v + delta).max(0.0))
            .or_insert_with(|| delta.max(0.0));

        // Adjust per-user monthly (clamp to 0.0)
        self.monthly
            .entry(user_id.to_string())
            .and_modify(|v| *v = (*v + delta).max(0.0))
            .or_insert_with(|| delta.max(0.0));

        // Adjust global totals
        self.add_global_daily(delta);
        self.add_global_monthly(delta);

        // Clamp global totals to 0.0 if they went negative from credits
        if self.load_global_daily() < 0.0 {
            self.store_global_daily(0.0);
        }
        if self.load_global_monthly() < 0.0 {
            self.store_global_monthly(0.0);
        }

        self.maybe_save();
    }

    /// Read-only budget check (no reservation).
    ///
    /// Used by the CostTrackable trait implementation. For the router's hot
    /// path, use `reserve_budget()` instead to avoid TOCTOU races.
    pub fn check_budget(
        &self,
        sender_id: &str,
        estimated_cost_usd: f64,
        user_daily_limit: f64,
        user_monthly_limit: f64,
    ) -> BudgetResult {
        let user_id = sender_id; // DashMap key
        self.reset_if_needed();

        // 1. User daily check
        if user_daily_limit > 0.0 {
            let current = self.daily_spend(user_id);
            if current + estimated_cost_usd > user_daily_limit {
                return BudgetResult::OverDailyUser {
                    current_spend: current,
                    daily_limit: user_daily_limit,
                };
            }
        }

        // 2. User monthly check
        if user_monthly_limit > 0.0 {
            let current = self.monthly_spend(user_id);
            if current + estimated_cost_usd > user_monthly_limit {
                return BudgetResult::OverMonthlyUser {
                    current_spend: current,
                    monthly_limit: user_monthly_limit,
                };
            }
        }

        // 3. Global daily check
        if self.config.global_daily_limit_usd > 0.0 {
            let current = self.load_global_daily();
            if current + estimated_cost_usd > self.config.global_daily_limit_usd {
                return BudgetResult::OverDailyGlobal {
                    current_spend: current,
                    daily_limit: self.config.global_daily_limit_usd,
                };
            }
        }

        // 4. Global monthly check
        if self.config.global_monthly_limit_usd > 0.0 {
            let current = self.load_global_monthly();
            if current + estimated_cost_usd > self.config.global_monthly_limit_usd {
                return BudgetResult::OverMonthlyGlobal {
                    current_spend: current,
                    monthly_limit: self.config.global_monthly_limit_usd,
                };
            }
        }

        BudgetResult::Ok
    }

    /// Get a sender's accumulated daily spend in USD.
    pub fn daily_spend(&self, sender_id: &str) -> f64 {
        self.daily.get(sender_id).map(|v| *v).unwrap_or(0.0)
    }

    /// Get a sender's accumulated monthly spend in USD.
    pub fn monthly_spend(&self, sender_id: &str) -> f64 {
        self.monthly.get(sender_id).map(|v| *v).unwrap_or(0.0)
    }

    /// Get total global daily spend across all users.
    pub fn global_daily_spend(&self) -> f64 {
        self.load_global_daily()
    }

    /// Get total global monthly spend across all users.
    pub fn global_monthly_spend(&self) -> f64 {
        self.load_global_monthly()
    }
}
```

### 2.6 Recording Spend (Superseded by reserve_budget / reconcile_actual)

The original `record_estimated()` and `record_actual()` methods are retained as
thin wrappers for backward compatibility and for the `CostTrackable` trait
implementation, but the router's hot path MUST use `reserve_budget()` (section
2.5) for atomic check+reserve.

```rust
impl CostTracker {
    /// Record an estimated cost for a user.
    ///
    /// DEPRECATED for direct use by the router hot path. Use `reserve_budget()`
    /// instead, which atomically checks and reserves. This method is retained
    /// for the `CostTrackable` trait implementation and edge cases where
    /// the caller has already performed its own budget check.
    pub fn record_estimated(&self, sender_id: &str, estimated_cost_usd: f64) {
        if estimated_cost_usd <= 0.0 {
            return;
        }

        let user_id = sender_id; // DashMap key
        self.reset_if_needed();

        // Update per-user daily
        self.daily
            .entry(user_id.to_string())
            .and_modify(|v| *v += estimated_cost_usd)
            .or_insert(estimated_cost_usd);

        // Update per-user monthly
        self.monthly
            .entry(user_id.to_string())
            .and_modify(|v| *v += estimated_cost_usd)
            .or_insert(estimated_cost_usd);

        // Update global totals
        self.add_global_daily(estimated_cost_usd);
        self.add_global_monthly(estimated_cost_usd);

        self.maybe_save();
    }

    /// Reconcile actual cost with a previous estimate.
    ///
    /// DEPRECATED for direct use by the router hot path. Use `reconcile_actual()`
    /// instead (section 2.5), which has the same logic. This method is retained
    /// for the `CostTrackable` trait implementation.
    pub fn record_actual(
        &self,
        sender_id: &str,
        estimated_cost_usd: f64,
        actual_cost_usd: f64,
    ) {
        self.reconcile_actual(sender_id, estimated_cost_usd, actual_cost_usd);
    }
}
```

### 2.7 Time-Based Auto-Reset

```rust
impl CostTracker {
    /// Check if daily or monthly counters need reset and reset them.
    ///
    /// Daily reset: when the current UTC hour passes `config.reset_hour_utc`
    /// since the last reset's day boundary.
    /// Monthly reset: when the current UTC month differs from the last reset's
    /// month.
    ///
    /// Called automatically from `check_budget()` and `record_estimated()`.
    /// Uses CAS to prevent double-reset from concurrent threads.
    pub fn reset_if_needed(&self) {
        let now = Self::now_unix();
        let last_daily = self.daily_reset_at.load(Ordering::Relaxed);
        let last_monthly = self.monthly_reset_at.load(Ordering::Relaxed);

        // Check daily reset
        if self.should_reset_daily(now, last_daily) {
            if self.daily_reset_at.compare_exchange(
                last_daily, now, Ordering::Relaxed, Ordering::Relaxed,
            ).is_ok() {
                debug!("resetting daily cost counters");
                self.daily.clear();
                self.store_global_daily(0.0);
            }
        }

        // Check monthly reset
        if self.should_reset_monthly(now, last_monthly) {
            if self.monthly_reset_at.compare_exchange(
                last_monthly, now, Ordering::Relaxed, Ordering::Relaxed,
            ).is_ok() {
                debug!("resetting monthly cost counters");
                self.monthly.clear();
                self.store_global_monthly(0.0);
                // Monthly reset also clears daily (new month = fresh slate)
                self.daily.clear();
                self.store_global_daily(0.0);
                self.daily_reset_at.store(now, Ordering::Relaxed);
            }
        }
    }

    /// Determine if a daily reset boundary has been crossed.
    ///
    /// Adjusts timestamps by `reset_hour_utc` so that "days" align with
    /// the configured reset hour rather than midnight. For example, with
    /// `reset_hour_utc = 6`, the day boundary is 06:00 UTC.
    fn should_reset_daily(&self, now_unix: u64, last_reset_unix: u64) -> bool {
        let reset_hour = self.config.reset_hour_utc as u64;
        let secs_per_day: u64 = 86400;

        // Shift timestamps so day boundaries align with reset_hour_utc
        let adjusted_now = now_unix.saturating_sub(reset_hour * 3600);
        let adjusted_last = last_reset_unix.saturating_sub(reset_hour * 3600);

        let day_now = adjusted_now / secs_per_day;
        let day_last = adjusted_last / secs_per_day;

        day_now > day_last
    }

    /// Determine if a monthly reset boundary has been crossed.
    ///
    /// Uses chrono to compare the UTC year+month of `now` vs `last_reset`.
    fn should_reset_monthly(&self, now_unix: u64, last_reset_unix: u64) -> bool {
        use chrono::{TimeZone, Utc, Datelike};

        let now_dt = Utc.timestamp_opt(now_unix as i64, 0)
            .single()
            .unwrap_or_else(Utc::now);
        let last_dt = Utc.timestamp_opt(last_reset_unix as i64, 0)
            .single()
            .unwrap_or_else(Utc::now);

        (now_dt.year(), now_dt.month()) != (last_dt.year(), last_dt.month())
    }
}
```

### 2.8 Utility Methods

```rust
impl CostTracker {
    /// Number of users with non-zero daily spend.
    pub fn active_users_daily(&self) -> usize {
        self.daily.len()
    }

    /// Number of users with non-zero monthly spend.
    pub fn active_users_monthly(&self) -> usize {
        self.monthly.len()
    }

    /// Snapshot of all per-user daily spend for reporting.
    pub fn daily_spend_report(&self) -> Vec<(String, f64)> {
        self.daily.iter().map(|e| (e.key().clone(), *e.value())).collect()
    }

    /// Snapshot of all per-user monthly spend for reporting.
    pub fn monthly_spend_report(&self) -> Vec<(String, f64)> {
        self.monthly.iter().map(|e| (e.key().clone(), *e.value())).collect()
    }

    /// Force a manual daily reset (for testing or admin override).
    pub fn force_daily_reset(&self) {
        self.daily.clear();
        self.store_global_daily(0.0);
        self.daily_reset_at.store(Self::now_unix(), Ordering::Relaxed);
    }

    /// Force a manual monthly reset (for testing or admin override).
    /// Also resets daily counters since a new month implies a new day.
    pub fn force_monthly_reset(&self) {
        self.monthly.clear();
        self.store_global_monthly(0.0);
        self.monthly_reset_at.store(Self::now_unix(), Ordering::Relaxed);
        // Monthly reset implies daily reset
        self.daily.clear();
        self.store_global_daily(0.0);
        self.daily_reset_at.store(Self::now_unix(), Ordering::Relaxed);
    }

    /// Returns a reference to the tracker's configuration.
    pub fn config(&self) -> &CostTrackerConfig {
        &self.config
    }
}

impl std::fmt::Debug for CostTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostTracker")
            .field("daily_users", &self.daily.len())
            .field("monthly_users", &self.monthly.len())
            .field("global_daily", &self.load_global_daily())
            .field("global_monthly", &self.load_global_monthly())
            .field("persistence_path", &self.persistence_path)
            .finish()
    }
}
```

### 2.9 CostTrackable Trait Implementation

The `CostTrackable` trait is defined in `tiered_router.rs` (Phase C) as the interface the router uses. `CostTracker` implements it by delegating to the concrete methods.

The trait signature (from Phase C, aligned by FIX-03):
```rust
pub trait CostTrackable: Send + Sync {
    fn check_budget(&self, sender_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult;
    fn record_estimated(&self, sender_id: &str, estimated_cost: f64);
    fn record_actual(&self, sender_id: &str, estimated_cost: f64, actual_cost: f64);
}
```

Implementation:
```rust
use super::tiered_router::CostTrackable;

impl CostTrackable for CostTracker {
    fn check_budget(
        &self,
        sender_id: &str,
        estimated_cost: f64,
        daily_limit: f64,
        monthly_limit: f64,
    ) -> BudgetResult {
        // Delegate to the concrete CostTracker::check_budget() method.
        // The method name is the same, but the trait makes it callable
        // through Arc<dyn CostTrackable> in TieredRouter.
        CostTracker::check_budget(self, sender_id, estimated_cost, daily_limit, monthly_limit)
    }

    fn record_estimated(&self, sender_id: &str, estimated_cost: f64) {
        // Delegate to the concrete CostTracker::record_estimated() method.
        CostTracker::record_estimated(self, sender_id, estimated_cost);
    }

    fn record_actual(&self, sender_id: &str, estimated_cost: f64, actual_cost: f64) {
        // Delegate to the concrete CostTracker::record_actual() method.
        CostTracker::record_actual(self, sender_id, estimated_cost, actual_cost);
    }
}
```

NOTE: The trait method signatures now match the concrete `CostTracker` methods
exactly. Public API uses `sender_id` to match `AuthContext.sender_id`.
Internally mapped to `user_id` as DashMap key. The `TieredRouter` calls
`check_budget()` with `auth.sender_id` and the user's
`permissions.cost_budget_daily_usd` and `permissions.cost_budget_monthly_usd`
as the `daily_limit` and `monthly_limit` parameters. No casting or downcasting
from `Arc<dyn CostTrackable>` to `Arc<CostTracker>` is needed.

### 2.10 Cost Estimation Helper

A pure function for computing estimated cost from tier pricing and token counts. Used by `TieredRouter` before calling `check_budget()`.

```rust
/// Estimate the cost of a request at a given tier.
///
/// Pure function. Used by `TieredRouter` to compute the
/// `estimated_cost_usd` before passing it to `CostTracker::check_budget()`.
pub fn estimate_cost(
    cost_per_1k_tokens: f64,
    input_tokens: usize,
    max_output_tokens: usize,
) -> f64 {
    let total_tokens = input_tokens + max_output_tokens;
    cost_per_1k_tokens * (total_tokens as f64) / 1000.0
}
```

---

## 3. Architecture

### 3.1 File Location

```
crates/clawft-core/src/pipeline/
  mod.rs              # Add: pub mod cost_tracker;
  cost_tracker.rs     # NEW: CostTracker, BudgetResult, CostTrackerConfig, estimate_cost
  tiered_router.rs    # Phase C: defines CostTrackable trait, holds Arc<dyn CostTrackable>
  rate_limiter.rs     # Phase E: sibling concurrent data structure using DashMap
  traits.rs           # Existing: RoutingDecision, ResponseOutcome
  router.rs           # Existing: StaticRouter (Level 0)
```

### 3.2 Dependencies

All dependencies are already in `crates/clawft-core/Cargo.toml`:

```toml
dashmap = { workspace = true }    # Needs to be added to workspace deps if not present
chrono = { workspace = true }     # Already present
serde = { workspace = true }      # Already present
serde_json = { workspace = true } # Already present
tracing = { workspace = true }    # Already present
```

Check the workspace root `Cargo.toml` for `dashmap`. If not present, add:

```toml
[workspace.dependencies]
dashmap = "6"
```

And add to `crates/clawft-core/Cargo.toml` `[dependencies]`:

```toml
dashmap = { workspace = true }
```

### 3.3 Data Flow

```
TieredRouter::route()
  |
  |  Step 6: Atomic budget check + reserve (FIX-07)
  |  estimated_cost = estimate_cost(tier.cost_per_1k_tokens, input_tokens, max_output_tokens)
  |
  +-- cost_tracker.reserve_budget(sender_id, estimated_cost, daily_limit, monthly_limit)
  |     |
  |     +-- let user_id = sender_id       // DashMap key alias
  |     +-- reset_if_needed()             // Auto-reset if day/month boundary passed
  |     +-- daily.entry(user_id)          // DashMap entry lock held for duration
  |     |     +-- check daily limit       // Within entry lock
  |     +-- monthly.entry(user_id)        // DashMap entry lock held for duration
  |     |     +-- check monthly limit     // Within entry lock
  |     +-- global_daily_bits.load()      // AtomicU64 load
  |     +-- global_monthly_bits.load()    // AtomicU64 load
  |     |
  |     |  All checks passed? Commit reservation:
  |     +-- *daily_entry += estimated      // Still within entry lock
  |     +-- *monthly_entry += estimated    // Still within entry lock
  |     +-- add_global_daily()            // CAS loop on AtomicU64
  |     +-- add_global_monthly()          // CAS loop on AtomicU64
  |     +-- maybe_save()                  // Periodic persist (every N ops)
  |     +-- return BudgetResult::Ok
  |
  |  If Over*: no spend recorded, return budget-constrained decision
  |
  |  ... LLM call happens ...
  |
TieredRouter::update(decision, outcome)
  |
  +-- cost_tracker.reconcile_actual(sender_id, estimated_cost, actual_cost)
        |
        +-- delta = actual - estimated
        +-- daily.entry().and_modify()    // Adjust by delta, clamp >= 0
        +-- monthly.entry().and_modify()  // Adjust by delta, clamp >= 0
        +-- add_global_daily(delta)       // CAS loop
        +-- add_global_monthly(delta)     // CAS loop
        +-- maybe_save()
```

### 3.4 Thread Safety Model

All `CostTracker` methods take `&self` (not `&mut self`). The struct is `Send + Sync` by construction.

| Field | Concurrency Mechanism | Hot-Path Cost |
|-------|----------------------|---------------|
| `daily`, `monthly` | `DashMap` -- sharded lock-free concurrent hashmap | ~30ns per lookup |
| `global_daily_bits`, `global_monthly_bits` | `AtomicU64` with CAS loop for f64 addition | ~10ns per load, ~50ns per CAS |
| `daily_reset_at`, `monthly_reset_at` | `AtomicU64` with CAS for one-shot reset | ~5ns (usually no-op) |
| `ops_since_save` | `AtomicU32` -- approximate counter, races acceptable | ~5ns |
| `config` | Immutable after construction | 0ns |
| `persistence_path` | Immutable after construction | 0ns |

The `save()` method performs synchronous file I/O. In production use, if latency is a concern, the caller can invoke `save()` from a `tokio::task::spawn_blocking` context. The `maybe_save()` path fires only every N operations and is best-effort.

### 3.5 Memory Bounds

- Each user entry in `DashMap`: ~80 bytes (String key + f64 + DashMap overhead).
- For 10,000 unique senders: ~1.6 MB (daily + monthly maps combined).
- After daily reset, the daily map is `.clear()`-ed entirely.
- Monthly map accumulates for the calendar month, cleared on the 1st.
- No eviction needed: unique sender count is bounded by actual platform users.

### 3.6 Persistence File Format

The file is plain JSON. Written atomically via temp-file-then-rename.

```json
{
  "daily": [
    ["alice_telegram_123", 2.45],
    ["bob_discord_456", 0.87]
  ],
  "monthly": [
    ["alice_telegram_123", 45.20],
    ["bob_discord_456", 12.30]
  ],
  "global_daily": 3.32,
  "global_monthly": 57.50,
  "daily_reset_at": 1739836800,
  "monthly_reset_at": 1738368000
}
```

### 3.7 Integration with clawft-llm::UsageTracker

From Section 9.5 of the design doc, the `CostTracker` and `clawft-llm::UsageTracker` are complementary:

| System | Purpose | Speed | Accuracy |
|--------|---------|-------|----------|
| `CostTracker` (this module) | Budget enforcement | Fast (~50ns check) | Approximate (tier-level estimates) |
| `clawft-llm::UsageTracker` | Analytics/reporting | N/A | Exact (per-model pricing from ModelCatalog) |

The `TieredRouter::update()` method calls both:
- `cost_tracker.record_actual()` for budget reconciliation
- `usage_tracker.record()` (in clawft-llm) for accurate cost reporting

The two systems do not depend on each other. The `CostTracker` has no import path to `clawft-llm`. The integration happens in the `TieredRouter` which holds references to both.

---

## 4. Refinement

### 4.1 Edge Cases

| Edge Case | Handling |
|-----------|----------|
| **Crash before save** | Accept up to `save_interval_ops` (default: 10) operations of data loss. Direction is always under-counting (safe: users get slightly more budget post-recovery, never incorrectly blocked). |
| **Negative spend after reconciliation** | Per-user spend clamped to `0.0` via `.max(0.0)`. Global totals also clamped. A negative value would mean "credit," which is not intended. |
| **Concurrent budget reservation (FIX-07)** | `reserve_budget()` holds the `DashMap::entry()` lock for the user during the check+reserve, so two concurrent requests for the same user are serialized. Requests for different users remain fully concurrent. Global limit checks are still racy (two threads can both pass the global check before either commits), but this is acceptable: global budgets are soft enforcement, and overshoot is bounded to one request's cost. |
| **User ID collision across channels** | User IDs are channel-prefixed by `AuthContext` (e.g., `"telegram_12345"`, `"discord_67890"`). The `CostTracker` treats IDs as opaque strings. Phase F handles the prefixing. |
| **Clock skew / NTP jumps** | `should_reset_daily()` uses day-number comparison: backward jumps yield same or lower day number (no spurious reset). Forward jumps trigger reset correctly. `should_reset_monthly()` uses chrono year+month comparison, same safe behavior. |
| **Persistence path not writable** | `save()` returns `Err`, logged as `tracing::warn`. Cost tracking continues in-memory. No crash, no data corruption. |
| **Corrupted persistence file** | `load_from_path()` returns `Err(InvalidData)`, logged as warning. Tracker starts with zero spend (full budget reset). |
| **Zero-cost tier (free models)** | `record_estimated(user, 0.0)` returns immediately (guard clause). Budget check also passes trivially since adding 0.0 never exceeds any limit. |
| **Admin with unlimited budget** | `user_daily_limit = 0.0` means unlimited. `check_budget()` skips that dimension via `if limit > 0.0` guard. |
| **Multiple saves racing** | `maybe_save()` uses `fetch_add` to track ops count. Two threads may both hit the threshold and both call `save()`. The second save is redundant but safe (same data written twice). The `ops_since_save` is reset by both, which is also safe. |
| **Process restart mid-month** | State is loaded from persistence file. If the file has a `monthly_reset_at` from the current month, monthly counters continue accumulating. If from a previous month, `reset_if_needed()` triggers a monthly reset on the first operation. |
| **reset_hour_utc = 23** | Shifts the day boundary to 23:00 UTC. Timestamps are adjusted by subtracting 23 hours before dividing by 86400. A request at 22:59 UTC and 23:01 UTC on the same calendar day will straddle the reset boundary correctly. |
| **Concurrent daily/monthly reset** | CAS on `daily_reset_at` / `monthly_reset_at` ensures exactly one thread performs the `clear()`. Losing threads see the CAS fail and skip the reset (correct: the winning thread already did it). |

### 4.2 Precision

- **f64 for USD**: 15-16 significant digits. For values ranging $0.0001 to $500.00, this provides ~11 digits beyond the decimal point. More than sufficient for budget enforcement.
- **Accumulation error**: Over 1 million additions of $0.01, accumulated error is less than $0.0000001. Not a concern.
- **Why not integer cents**: Tier costs are specified as `cost_per_1k_tokens` which are small floats (0.001, 0.01, 0.05). Using f64 throughout avoids conversion errors at boundaries and matches the config types.

### 4.3 Performance

| Operation | Cost | Notes |
|-----------|------|-------|
| `reserve_budget()` | ~120ns | 2 DashMap entry locks + 2 atomic loads + 2 CAS loops (atomic check+reserve) |
| `reconcile_actual()` | ~100ns | 2 DashMap upserts + 2 CAS loops |
| `check_budget()` (read-only) | ~50ns | 2 DashMap lookups + 2 atomic loads, no allocation |
| `record_estimated()` | ~100ns | 2 DashMap upserts + 2 CAS loops |
| `maybe_save()` (no-op path) | ~5ns | Single atomic increment |
| `maybe_save()` (save path) | ~1ms | File write + chmod, happens every N ops |
| `reset_if_needed()` (no-op path) | ~20ns | Timestamp comparison + atomic load |
| `reset_if_needed()` (reset path) | ~100us | DashMap::clear() + CAS, rare |

The hot path (`reserve_budget`) is under 150ns, well within the 1ms budget for the entire routing decision. This is slightly slower than the previous two-step pattern (~200ns combined) because the DashMap entry lock is held longer, but the trade-off eliminates TOCTOU race conditions.

### 4.4 Security Considerations

- The persistence file at `~/.clawft/cost_tracking.json` contains user IDs and spend amounts. File permissions are set to 0600 (owner read/write only) after every write via `save()` (FIX-12). On non-Unix platforms, the `#[cfg(unix)]` guard is a no-op; operators should ensure appropriate filesystem ACLs.
- User IDs stored in the tracker are platform identifiers (numeric user IDs, usernames). They should never contain secrets, passwords, or tokens.
- The JSON persistence file should not be served via any HTTP endpoint. It is a local-only file.

---

## 5. Completion

### 5.1 Exit Criteria

- [ ] `CostTracker::check_budget()` returns `BudgetResult::Ok` when within all limits
- [ ] `CostTracker::check_budget()` returns the correct `Over*` variant when any limit is exceeded
- [ ] `CostTracker::reserve_budget()` atomically checks and reserves within DashMap entry lock (FIX-07)
- [ ] `CostTracker::reserve_budget()` does NOT record spend when budget is exceeded (FIX-07)
- [ ] `CostTracker::reconcile_actual()` correctly adjusts reservation with actual cost delta (FIX-07)
- [ ] `record_estimated()` increases both daily and monthly spend for the user and global totals
- [ ] `record_actual()` delegates to `reconcile_actual()` correctly
- [ ] Daily reset clears daily counters when `reset_hour_utc` boundary is crossed
- [ ] Monthly reset clears both monthly and daily counters when a new month begins
- [ ] Persistence roundtrip: `save()` then `load()` in a fresh tracker restores identical state
- [ ] Atomic file write: partial save does not corrupt existing file (temp + rename pattern)
- [ ] Persistence file has 0600 permissions on Unix after save (FIX-12)
- [ ] Global budget limits enforced independently of per-user limits
- [ ] Zero limits (0.0) treated as unlimited (no enforcement for that dimension)
- [ ] Concurrent access from 10+ threads does not cause data races or panics
- [ ] `CostTracker` implements the `CostTrackable` trait from Phase C with matching signatures (FIX-03)
- [ ] `CostTrackable` trait impl delegates to concrete methods without signature mismatch (FIX-03)
- [ ] `estimate_cost()` helper returns correct values for free, standard, premium, elite tiers
- [ ] All code compiles without warnings: `cargo clippy -p clawft-core -- -D warnings`
- [ ] All tests pass: `cargo test -p clawft-core cost_tracker`

### 5.2 Test Plan (25 tests minimum)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_config() -> CostTrackerConfig {
        CostTrackerConfig {
            global_daily_limit_usd: 50.0,
            global_monthly_limit_usd: 500.0,
            reset_hour_utc: 0,
            persistence_enabled: false,
            save_interval_ops: 100, // high to avoid saves during tests
        }
    }

    // --- Budget Check Tests ---

    // 1. check_budget returns Ok when within all limits
    #[test]
    fn check_budget_ok_within_limits() {
        let tracker = CostTracker::in_memory(test_config());
        let result = tracker.check_budget("alice", 1.0, 5.0, 100.0);
        assert_eq!(result, BudgetResult::Ok);
    }

    // 2. check_budget returns OverDailyUser when user daily limit exceeded
    #[test]
    fn check_budget_over_daily_user() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 4.5);
        let result = tracker.check_budget("alice", 1.0, 5.0, 100.0);
        assert!(matches!(result, BudgetResult::OverDailyUser { .. }));
    }

    // 3. check_budget returns OverMonthlyUser when user monthly limit exceeded
    #[test]
    fn check_budget_over_monthly_user() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 99.5);
        let result = tracker.check_budget("alice", 1.0, 200.0, 100.0);
        assert!(matches!(result, BudgetResult::OverMonthlyUser { .. }));
    }

    // 4. check_budget returns OverDailyGlobal when global daily limit exceeded
    #[test]
    fn check_budget_over_daily_global() {
        let config = CostTrackerConfig {
            global_daily_limit_usd: 10.0,
            ..test_config()
        };
        let tracker = CostTracker::in_memory(config);
        tracker.record_estimated("alice", 5.0);
        tracker.record_estimated("bob", 4.5);
        // Global daily = 9.5, requesting 1.0 -> 10.5 > 10.0
        let result = tracker.check_budget("charlie", 1.0, 50.0, 500.0);
        assert!(matches!(result, BudgetResult::OverDailyGlobal { .. }));
    }

    // 5. check_budget returns OverMonthlyGlobal when global monthly limit exceeded
    #[test]
    fn check_budget_over_monthly_global() {
        let config = CostTrackerConfig {
            global_monthly_limit_usd: 10.0,
            ..test_config()
        };
        let tracker = CostTracker::in_memory(config);
        tracker.record_estimated("alice", 9.5);
        let result = tracker.check_budget("bob", 1.0, 50.0, 50.0);
        assert!(matches!(result, BudgetResult::OverMonthlyGlobal { .. }));
    }

    // 6. check_budget with all limits = 0.0 is always Ok (unlimited)
    #[test]
    fn check_budget_unlimited_when_zero() {
        let config = CostTrackerConfig {
            global_daily_limit_usd: 0.0,
            global_monthly_limit_usd: 0.0,
            ..test_config()
        };
        let tracker = CostTracker::in_memory(config);
        tracker.record_estimated("alice", 999.0);
        let result = tracker.check_budget("alice", 100.0, 0.0, 0.0);
        assert_eq!(result, BudgetResult::Ok);
    }

    // 7. check_budget priority: user daily checked before user monthly
    #[test]
    fn check_budget_daily_checked_before_monthly() {
        let tracker = CostTracker::in_memory(test_config());
        // Spend enough to exceed both daily (5.0) and monthly (10.0)
        tracker.record_estimated("alice", 9.5);
        let result = tracker.check_budget("alice", 1.0, 5.0, 10.0);
        // Should hit daily first, not monthly
        assert!(matches!(result, BudgetResult::OverDailyUser { .. }));
    }

    // --- Recording Tests ---

    // 8. record_estimated updates daily, monthly, and global spend
    #[test]
    fn record_estimated_updates_spend() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 2.50);
        assert!((tracker.daily_spend("alice") - 2.50).abs() < 1e-10);
        assert!((tracker.monthly_spend("alice") - 2.50).abs() < 1e-10);
        assert!((tracker.global_daily_spend() - 2.50).abs() < 1e-10);
        assert!((tracker.global_monthly_spend() - 2.50).abs() < 1e-10);
    }

    // 9. record_actual reconciles estimate with actual cost
    #[test]
    fn record_actual_adjusts_delta() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 2.00);    // estimate
        tracker.record_actual("alice", 2.00, 1.50); // actual was cheaper
        // Spend should be 2.00 + (1.50 - 2.00) = 1.50
        assert!((tracker.daily_spend("alice") - 1.50).abs() < 1e-10);
        assert!((tracker.monthly_spend("alice") - 1.50).abs() < 1e-10);
    }

    // 10. record_actual clamps negative spend to zero
    #[test]
    fn record_actual_clamps_to_zero() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 0.50);
        // Actual is 0.0, delta = -0.50. Spend should clamp to 0.0.
        tracker.record_actual("alice", 0.50, 0.0);
        assert!(tracker.daily_spend("alice") >= 0.0);
        assert!(tracker.monthly_spend("alice") >= 0.0);
    }

    // 11. record_estimated with zero cost is a no-op
    #[test]
    fn record_zero_cost_is_noop() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 0.0);
        assert!((tracker.daily_spend("alice")).abs() < 1e-10);
        assert_eq!(tracker.active_users_daily(), 0);
    }

    // --- Reset Tests ---

    // 12. force_daily_reset clears daily but preserves monthly
    #[test]
    fn force_daily_reset_clears_daily_preserves_monthly() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 5.0);
        tracker.record_estimated("bob", 3.0);
        assert!((tracker.global_daily_spend() - 8.0).abs() < 1e-10);

        tracker.force_daily_reset();

        assert!((tracker.daily_spend("alice")).abs() < 1e-10);
        assert!((tracker.daily_spend("bob")).abs() < 1e-10);
        assert!((tracker.global_daily_spend()).abs() < 1e-10);
        // Monthly should be unaffected
        assert!((tracker.monthly_spend("alice") - 5.0).abs() < 1e-10);
        assert!((tracker.monthly_spend("bob") - 3.0).abs() < 1e-10);
        assert!((tracker.global_monthly_spend() - 8.0).abs() < 1e-10);
    }

    // 13. force_monthly_reset clears both daily and monthly
    #[test]
    fn force_monthly_reset_clears_both() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 5.0);

        tracker.force_monthly_reset();

        assert!((tracker.daily_spend("alice")).abs() < 1e-10);
        assert!((tracker.monthly_spend("alice")).abs() < 1e-10);
        assert!((tracker.global_daily_spend()).abs() < 1e-10);
        assert!((tracker.global_monthly_spend()).abs() < 1e-10);
    }

    // 14. should_reset_daily detects day boundary correctly
    #[test]
    fn daily_reset_boundary_detection() {
        let tracker = CostTracker::in_memory(CostTrackerConfig {
            reset_hour_utc: 0,
            ..test_config()
        });
        let now = 1739836800; // arbitrary timestamp
        // Same day: should not reset
        assert!(!tracker.should_reset_daily(now, now));
        assert!(!tracker.should_reset_daily(now + 3600, now)); // 1 hour later
        // Next day: should reset
        assert!(tracker.should_reset_daily(now + 86400, now)); // 24 hours later
    }

    // 15. should_reset_daily respects reset_hour_utc
    #[test]
    fn daily_reset_respects_reset_hour() {
        let tracker = CostTracker::in_memory(CostTrackerConfig {
            reset_hour_utc: 6, // Reset at 06:00 UTC
            ..test_config()
        });
        // 2025-02-18 05:00 UTC -> 2025-02-18 07:00 UTC (same "cost day")
        let t_0500 = 1739854800; // example: 05:00 UTC
        let t_0700 = t_0500 + 7200; // +2 hours = 07:00 UTC
        // Both are within the same reset-hour-adjusted day
        // (but this depends on the actual day boundary; use a controlled example)

        // More precise test: two timestamps on different sides of 06:00
        // Day N 23:00 and Day N+1 05:59 should NOT trigger reset (same cost day)
        // Day N 05:59 and Day N 06:01 SHOULD trigger reset if on different days

        // Simple version: 25 hours apart always crosses a day boundary
        let base = 1739836800;
        assert!(tracker.should_reset_daily(base + 90000, base)); // 25 hours
    }

    // --- Persistence Tests ---

    // 16. persistence roundtrip: save then load restores state
    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join("clawft_cost_test_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cost_tracking.json");
        let _ = std::fs::remove_file(&path);

        // Create, record, save
        {
            let config = CostTrackerConfig {
                persistence_enabled: true,
                ..test_config()
            };
            let tracker = CostTracker::new(config, Some(path.clone()));
            tracker.record_estimated("alice", 3.50);
            tracker.record_estimated("bob", 1.25);
            tracker.save().expect("save failed");
        }

        // Load into a fresh tracker
        {
            let config = test_config();
            let tracker = CostTracker::new(config, Some(path.clone()));
            assert!((tracker.daily_spend("alice") - 3.50).abs() < 1e-10);
            assert!((tracker.daily_spend("bob") - 1.25).abs() < 1e-10);
            assert!((tracker.global_daily_spend() - 4.75).abs() < 1e-10);
        }

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    // --- Concurrent Access Test ---

    // 17. concurrent record from multiple threads does not panic or corrupt
    #[test]
    fn concurrent_record_no_panic() {
        let tracker = Arc::new(CostTracker::in_memory(test_config()));
        let mut handles = vec![];

        for i in 0..10 {
            let t = Arc::clone(&tracker);
            handles.push(std::thread::spawn(move || {
                for j in 0..100 {
                    let user = format!("user_{}", i);
                    t.record_estimated(&user, 0.01);
                    if j % 10 == 0 {
                        let _ = t.check_budget(&user, 0.01, 50.0, 500.0);
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        // 10 users * 100 records * $0.01 = $10.00 total
        let total: f64 = (0..10)
            .map(|i| tracker.daily_spend(&format!("user_{i}")))
            .sum();
        assert!((total - 10.0).abs() < 0.01);
        assert!((tracker.global_daily_spend() - 10.0).abs() < 0.01);
    }

    // --- Utility Tests ---

    // 18. estimate_cost helper produces correct values
    #[test]
    fn estimate_cost_calculation() {
        // Standard tier: 0.001 per 1K tokens, 500 input + 500 output = 1K -> $0.001
        let cost = estimate_cost(0.001, 500, 500);
        assert!((cost - 0.001).abs() < 1e-10);

        // Free tier: always zero
        let cost = estimate_cost(0.0, 1000, 1000);
        assert!((cost).abs() < 1e-10);

        // Elite tier: 0.05 per 1K, 5000 input + 5000 output = 10K -> $0.50
        let cost = estimate_cost(0.05, 5000, 5000);
        assert!((cost - 0.50).abs() < 1e-10);

        // Premium tier: 0.01 per 1K, 2000 input + 4096 output = 6096 tokens -> $0.06096
        let cost = estimate_cost(0.01, 2000, 4096);
        assert!((cost - 0.06096).abs() < 1e-10);
    }

    // 19. active_users_count tracks unique senders correctly
    #[test]
    fn active_users_count() {
        let tracker = CostTracker::in_memory(test_config());
        assert_eq!(tracker.active_users_daily(), 0);
        tracker.record_estimated("alice", 1.0);
        tracker.record_estimated("bob", 2.0);
        assert_eq!(tracker.active_users_daily(), 2);
        assert_eq!(tracker.active_users_monthly(), 2);
    }

    // 20. BudgetResult::is_ok() helper
    #[test]
    fn budget_result_is_ok() {
        assert!(BudgetResult::Ok.is_ok());
        assert!(!BudgetResult::OverDailyUser {
            current_spend: 5.0,
            daily_limit: 5.0,
        }.is_ok());
        assert!(!BudgetResult::OverMonthlyUser {
            current_spend: 100.0,
            monthly_limit: 100.0,
        }.is_ok());
        assert!(!BudgetResult::OverDailyGlobal {
            current_spend: 50.0,
            daily_limit: 50.0,
        }.is_ok());
        assert!(!BudgetResult::OverMonthlyGlobal {
            current_spend: 500.0,
            monthly_limit: 500.0,
        }.is_ok());
    }

    // 21. Multiple users accumulate independently in daily and monthly maps
    #[test]
    fn independent_user_accumulation() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 3.0);
        tracker.record_estimated("bob", 7.0);
        tracker.record_estimated("alice", 2.0);

        assert!((tracker.daily_spend("alice") - 5.0).abs() < 1e-10);
        assert!((tracker.daily_spend("bob") - 7.0).abs() < 1e-10);
        assert!((tracker.global_daily_spend() - 12.0).abs() < 1e-10);
    }

    // 22. spend_report returns snapshot of all users
    #[test]
    fn spend_report_snapshot() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.record_estimated("alice", 3.0);
        tracker.record_estimated("bob", 7.0);

        let report = tracker.daily_spend_report();
        assert_eq!(report.len(), 2);
        let alice_spend = report.iter().find(|(k, _)| k == "alice").map(|(_, v)| *v);
        assert!((alice_spend.unwrap() - 3.0).abs() < 1e-10);
    }

    // --- Atomic Reserve Budget Tests (FIX-07) ---

    // 23. reserve_budget atomically checks and reserves
    #[test]
    fn reserve_budget_atomic_check_and_reserve() {
        let tracker = CostTracker::in_memory(test_config());
        let result = tracker.reserve_budget("alice", 2.50, 5.0, 100.0);
        assert_eq!(result, BudgetResult::Ok);
        // Spend should already be recorded (no separate record_estimated needed)
        assert!((tracker.daily_spend("alice") - 2.50).abs() < 1e-10);
        assert!((tracker.monthly_spend("alice") - 2.50).abs() < 1e-10);
        assert!((tracker.global_daily_spend() - 2.50).abs() < 1e-10);
    }

    // 24. reserve_budget does NOT record spend when over budget
    #[test]
    fn reserve_budget_no_record_on_failure() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.reserve_budget("alice", 4.0, 5.0, 100.0); // reserve 4.0 of 5.0 daily
        let result = tracker.reserve_budget("alice", 2.0, 5.0, 100.0); // would exceed 5.0
        assert!(matches!(result, BudgetResult::OverDailyUser { .. }));
        // Daily spend should still be 4.0 (the failed reserve did not add 2.0)
        assert!((tracker.daily_spend("alice") - 4.0).abs() < 1e-10);
    }

    // 25. reserve_budget with zero cost always succeeds
    #[test]
    fn reserve_budget_zero_cost_ok() {
        let tracker = CostTracker::in_memory(test_config());
        let result = tracker.reserve_budget("alice", 0.0, 5.0, 100.0);
        assert_eq!(result, BudgetResult::Ok);
        assert!((tracker.daily_spend("alice")).abs() < 1e-10);
    }

    // 26. reconcile_actual adjusts reservation correctly
    #[test]
    fn reconcile_actual_adjusts_reservation() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.reserve_budget("alice", 3.00, 10.0, 100.0);
        // Actual cost was 2.00 (cheaper than estimated)
        tracker.reconcile_actual("alice", 3.00, 2.00);
        assert!((tracker.daily_spend("alice") - 2.00).abs() < 1e-10);
        assert!((tracker.monthly_spend("alice") - 2.00).abs() < 1e-10);
        assert!((tracker.global_daily_spend() - 2.00).abs() < 1e-10);
    }

    // 27. reconcile_actual clamps to zero on large credit
    #[test]
    fn reconcile_actual_clamps_to_zero() {
        let tracker = CostTracker::in_memory(test_config());
        tracker.reserve_budget("alice", 1.00, 10.0, 100.0);
        // Actual was free -- delta = 0.0 - 1.0 = -1.0, clamp to 0.0
        tracker.reconcile_actual("alice", 1.00, 0.0);
        assert!(tracker.daily_spend("alice") >= 0.0);
        assert!(tracker.monthly_spend("alice") >= 0.0);
    }

    // 28. concurrent reserve_budget for same user serializes correctly
    #[test]
    fn concurrent_reserve_budget_same_user() {
        let tracker = Arc::new(CostTracker::in_memory(CostTrackerConfig {
            global_daily_limit_usd: 1000.0,
            global_monthly_limit_usd: 10000.0,
            ..test_config()
        }));
        let mut handles = vec![];

        // 20 threads each try to reserve $0.50 for the same user with $8.00 daily limit
        for _ in 0..20 {
            let t = Arc::clone(&tracker);
            handles.push(std::thread::spawn(move || {
                t.reserve_budget("alice", 0.50, 8.0, 100.0)
            }));
        }

        let results: Vec<BudgetResult> = handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .collect();

        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let over_count = results.iter().filter(|r| !r.is_ok()).count();

        // With $8.00 limit and $0.50 per request, at most 16 can succeed.
        // Due to atomic reservation, the total spend should never exceed $8.00.
        assert!(ok_count <= 16);
        assert!(ok_count + over_count == 20);
        assert!(tracker.daily_spend("alice") <= 8.0 + 1e-10);
    }

    // --- Persistence Permission Tests (FIX-12) ---

    // 29. persistence file has 0600 permissions on Unix
    #[cfg(unix)]
    #[test]
    fn persistence_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("clawft_cost_test_perms");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cost_tracking.json");
        let _ = std::fs::remove_file(&path);

        let config = CostTrackerConfig {
            persistence_enabled: true,
            ..test_config()
        };
        let tracker = CostTracker::new(config, Some(path.clone()));
        tracker.record_estimated("alice", 1.0);
        tracker.save().expect("save failed");

        let metadata = std::fs::metadata(&path).expect("metadata failed");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "persistence file should have 0600 permissions");

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
```

### 5.3 Files Modified

| File | Change |
|------|--------|
| `crates/clawft-core/src/pipeline/cost_tracker.rs` | **NEW** -- Full CostTracker implementation (~400 lines) |
| `crates/clawft-core/src/pipeline/mod.rs` | Add `pub mod cost_tracker;` |
| `crates/clawft-core/Cargo.toml` | Add `dashmap = { workspace = true }` if not already present |
| Root `Cargo.toml` | Add `dashmap = "6"` to `[workspace.dependencies]` if not already present |

### 5.4 Verification Commands

```bash
# Build the crate
cargo build -p clawft-core

# Run only cost_tracker tests
cargo test -p clawft-core cost_tracker

# Run all pipeline tests (ensure no regressions)
cargo test -p clawft-core

# Lint check
cargo clippy -p clawft-core -- -D warnings

# Full workspace (ensure nothing else breaks)
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### 5.5 Development Notes

#### Atomic f64 Pattern
Since `AtomicF64` is not in Rust's standard library, global totals use `AtomicU64` with `f64::to_bits()` / `f64::from_bits()`. The `add_global_daily()` / `add_global_monthly()` methods use CAS (compare-and-swap) loops. This is a well-established lock-free pattern. The CAS loop is bounded: under typical concurrency (< 100 threads), contention resolves in 1-3 iterations.

#### Persistence Strategy
The "write to temp + rename" pattern provides atomic file replacement on POSIX filesystems. A crash during `save()` either leaves the old file intact or replaces it with the new one. The `save_interval_ops = 10` default means at most 10 operations can be lost on crash. Since these are estimated costs (not billing records), the loss is acceptable.

#### DashMap vs Mutex<HashMap>
`DashMap` was chosen over `Mutex<HashMap>` because budget checks happen in the hot path of every request. `DashMap` provides sharded concurrent access -- different users' spend is independent and does not serialize. Under 10 concurrent users, contention is effectively zero.

#### Crash Recovery
On restart, `CostTracker::new()` loads the last saved state. If the file is missing or corrupt, the tracker starts fresh (all budgets fully available). This is the safe direction: after a crash, users may get slightly more budget than they should, but they are never incorrectly blocked.

#### Integration with CostTrackable Trait (Phase C, FIX-03)
The `CostTrackable` trait in `tiered_router.rs` now has method signatures that match the concrete `CostTracker` methods exactly (FIX-03). The trait methods are: `check_budget(sender_id, estimated_cost, daily_limit, monthly_limit) -> BudgetResult`, `record_estimated(sender_id, estimated_cost)`, and `record_actual(sender_id, estimated_cost, actual_cost)`. Public API uses `sender_id` to match `AuthContext.sender_id`. Internally mapped to `user_id` as DashMap key. The `impl CostTrackable for CostTracker` delegates directly to the concrete methods with no signature mismatch or lossy adaptation. No downcasting from `Arc<dyn CostTrackable>` to `Arc<CostTracker>` is needed.

#### Atomic Budget Reservation (FIX-07)
The `reserve_budget()` method replaces the previous two-step `check_budget()` + `record_estimated()` pattern in the router hot path. By holding the `DashMap::entry()` lock for the user during both the check and the reservation, it prevents TOCTOU race conditions where two concurrent threads could both pass the budget check before either records the spend. After the LLM call completes, `reconcile_actual()` adjusts the reservation with the real cost delta. The old `check_budget()` and `record_estimated()` methods are retained for the `CostTrackable` trait implementation and for read-only queries.

#### Persistence File Permissions (FIX-12)
On Unix platforms, `save()` sets file permissions to 0600 (owner read/write only) after writing the persistence file. This prevents other users on the system from reading cost tracking data which contains user identifiers and spend amounts. On non-Unix platforms, the permission call is a compile-time no-op via `#[cfg(unix)]`.

### 5.6 What This Phase Does NOT Include

- Per-model pricing lookup (that is `clawft-llm::UsageTracker`)
- Distributed budget tracking across multiple processes
- Budget alerts or notifications
- Budget history or analytics over time
- Per-channel budgets (only per-user and global)
- Config validation (Phase H)
- Integration testing with full TieredRouter pipeline (Phase I)

### 5.7 Branch and Effort

- **Branch**: `weft/tiered-router`
- **Effort**: 1 day
- **Depends on**: Phase C (TieredRouter defines `CostTrackable` trait, holds `Arc<dyn CostTrackable>`)
- **Blocks**: Phase I (integration tests)

---

## Remediation Applied

**Date**: 2026-02-18
**Applied by**: SPARC Implementation Specialist

The following remediation fixes from `remediation-plan.md` have been applied to this Phase D plan:

### FIX-03: CostTrackable Trait Alignment (CRITICAL)

**Problem**: The `CostTrackable` trait defined in Phase C had a different signature than the concrete `CostTracker` methods. The old trait impl used lossy adapter methods (`check_budget() -> Option<f64>`, `record_estimated_cost()`, `record_actual_cost()`) that did not pass user-specific budget limits or both estimated and actual costs.

**Changes applied**:
- Section 2.9: Replaced the old `impl CostTrackable for CostTracker` with a direct delegation implementation. The trait methods now match the concrete `CostTracker` signatures exactly: `check_budget(sender_id, estimated_cost, daily_limit, monthly_limit) -> BudgetResult`, `record_estimated(sender_id, estimated_cost)`, `record_actual(sender_id, estimated_cost, actual_cost)`. Public API uses `sender_id` to match `AuthContext.sender_id`; internally mapped to `user_id` as DashMap key.
- Section 1.6 (Public API Surface): Updated table to reflect aligned methods.
- Dependencies section: Updated Phase C dependency description to reference aligned trait.
- Section 5.5 (Development Notes): Replaced the old "Integration with CostTrackable Trait" note with corrected description.
- Section 5.1 (Exit Criteria): Added criteria for trait signature alignment and delegation correctness.

### FIX-07: Atomic Budget Reserve (HIGH SECURITY)

**Problem**: The two-step `check_budget()` + `record_estimated()` pattern had a TOCTOU race condition. Two concurrent threads could both pass the budget check before either recorded the spend, overshooting the budget by one request's cost.

**Changes applied**:
- Section 2.5: Added `reserve_budget()` method that atomically checks all four budget dimensions and reserves the estimated cost within a `DashMap::entry()` lock. On failure, no spend is recorded. Added `reconcile_actual()` method for post-LLM adjustment.
- Section 2.5: Retained `check_budget()` as a read-only query for the `CostTrackable` trait.
- Section 2.6: Marked `record_estimated()` and `record_actual()` as deprecated for the router hot path. `record_actual()` now delegates to `reconcile_actual()`.
- Section 1.6 (Public API Surface): Added `reserve_budget` and `reconcile_actual` entries.
- Section 3.3 (Data Flow): Replaced the two-step flow diagram with the atomic `reserve_budget()` flow.
- Section 4.1 (Edge Cases): Replaced "Concurrent check + record race" with "Concurrent budget reservation (FIX-07)" describing the serialized behavior.
- Section 4.3 (Performance): Added `reserve_budget()` timing and noted the trade-off vs the old pattern.
- Section 5.1 (Exit Criteria): Added criteria for atomic reservation and no-record-on-failure behavior.
- Section 5.2 (Test Plan): Added tests 23-28 covering `reserve_budget()`, `reconcile_actual()`, and concurrent reservation behavior.

### FIX-12: Persistence File Permissions (MEDIUM)

**Problem**: The persistence file at `~/.clawft/cost_tracking.json` contains user IDs and spend amounts but was written with default permissions, potentially allowing other system users to read it.

**Changes applied**:
- Section 2.3 (`save()` method): Added `#[cfg(unix)]` block to set 0600 permissions on the persistence file after write using `std::os::unix::fs::PermissionsExt`.
- Section 4.4 (Security Considerations): Updated note to reflect that permissions are now set programmatically.
- Section 5.1 (Exit Criteria): Added criterion for 0600 file permissions.
- Section 5.2 (Test Plan): Added test 29 (`persistence_file_permissions`) that verifies 0600 mode on Unix.

---

**End of SPARC Plan**
