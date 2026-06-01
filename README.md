# lau-token-economy

> Token budget system for the PLATO agent ecosystem — as agents gain experience, they spend fewer tokens on the same tasks through abstraction discounts and muscle memory.

## What This Does

`lau-token-economy` models a token budget for AI agents where **experience makes you cheaper**. Instead of a flat rate for every operation, agents earn **abstraction levels** that unlock discounts (up to 75%), and learn **routines** that decay in cost with each use until they're free. The system tracks every token spent in a double-entry ledger with per-category breakdowns and efficiency reports.

The core insight: a journeyman doesn't think about how to hold the hammer. As agents internalize patterns, the same task should cost fewer context tokens.

---

## Key Idea

Token usage **decreases** as the agent gains experience in a domain:

- **Level 1:** Full cost. Verbose instructions, no shortcuts.
- **Level 2–5:** Progressive abstraction discounts (20%, 40%, 60%, 75%).
- **Muscle Memory:** Routines decay by 5% per use. Level 3 → half cost. Level 5 → free.

The economy composes three subsystems:

1. **TokenBudget** — hard cap on total token spend with reserve/release semantics.
2. **AbstractionSavings** — journeyman discount tiers tied to agent level.
3. **MuscleMemory** — routines that get cheaper every time you use them.

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-token-economy = "0.1"
```

Requires **Rust 2024 edition**.

---

## Quick Start

```rust
use lau_token_economy::*;

// 1. Create an economy with a budget
let mut economy = TokenEconomy::new("agent-42", 10_000);

// 2. Execute a task at full cost (Level 1)
let result = economy.execute_task("compile", "dev", 200);
println!("Cost: {} (saved: {})", result.actual_cost, result.savings);
// Cost: 200 (saved: 0)

// 3. Learn a routine
economy.learn_routine("deploy", "ops", 500);

// 4. Execute the same task — routine kicks in
let result = economy.execute_task("deploy", "ops", 500);
println!("Routine used: {:?}, Cost: {}", result.routine_used, result.actual_cost);
// Routine used: Some("deploy"), Cost: 475 (5% decay on first use)

// 5. Promote the agent — unlocks discounts
economy.promote(); // Level 1 → Level 2 (20% discount)
let result = economy.execute_task("build", "dev", 100);
println!("After promotion: cost {} (saved {})", result.actual_cost, result.savings);
// After promotion: cost 80 (saved 20)

// 6. Full economy report
println!("{}", economy.economy_report());
```

---

## API Reference

### TokenBudget

Tracks total, used, and reserved tokens for a session.

```rust
let mut budget = TokenBudget::new(1000);
budget.spend(200);       // → true, used = 200
budget.reserve(100);     // → true, reserved = 100
budget.remaining();      // 700
budget.release(50);      // reserved = 50
budget.utilization();    // 0.2
```

| Method | Description |
|--------|-------------|
| `new(total)` | Create budget with a hard cap. |
| `spend(tokens) → bool` | Spend tokens (fails if exceeds available). |
| `reserve(tokens) → bool` | Reserve tokens for future use. |
| `release(tokens)` | Release reserved tokens back. |
| `remaining() → u64` | `total - used - reserved`. |
| `utilization() → f64` | `used / total` (0.0 if total is 0). |

### TokenCategory

Enum of token expenditure categories:

`SystemPrompt`, `ContextLoad`, `ToolCall`, `ToolResult`, `ModelInference`, `MemoryRecall`, `Compilation`, `Abstraction`, `Delegation`.

### TokenTransaction

A single spend record with auto-generated ID, tick, category, and description.

### TokenLedger

Double-entry bookkeeping for all token transactions.

```rust
let mut ledger = TokenLedger::new(5000);
ledger.spend(100, TokenCategory::ModelInference, "gpt-4 call");
ledger.spend_for_task("compile", 50);  // Category = Compilation
ledger.tick();                          // Advance time

// Query
ledger.transactions_by_category(&TokenCategory::ModelInference);  // Vec<&TokenTransaction>
ledger.total_by_category(&TokenCategory::ModelInference);          // 100
ledger.category_breakdown();   // HashMap<String, u64>
ledger.efficiency_report();    // TokenEfficiencyReport
```

| Method | Description |
|--------|-------------|
| `new(budget_total)` | Create ledger with budget. |
| `spend(tokens, category, desc) → bool` | Record a spend. |
| `spend_for_task(task, tokens) → bool` | Spend attributed to a task (Compilation category). |
| `tick()` | Advance the time counter. |
| `transactions_by_category(cat)` | Filter transactions. |
| `total_by_category(cat) → u64` | Sum tokens in a category. |
| `category_breakdown() → HashMap<String, u64>` | All categories with totals. |
| `efficiency_report() → TokenEfficiencyReport` | Tokens/task, trend, most expensive category. |

### AbstractionSavings

Tracks how much abstraction saves at each agent level.

```rust
let mut savings = AbstractionSavings::new();
savings.record(3, 100, 60);  // Level 3, base 100, actual 60 → saved 40
savings.discount_for_level(3);   // 0.40
savings.apply_discount(3, 100);  // 60
savings.savings_report();        // Human-readable string
```

**Discount schedule:**

| Level | Discount | Example: 100 tokens |
|-------|----------|-------------------|
| 1 | 0% | 100 |
| 2 | 20% | 80 |
| 3 | 40% | 60 |
| 4 | 60% | 40 |
| 5 | 75% | 25 |
| 6+ | 75% (capped) | 25 |

### MuscleMemory

Routines that decay in cost with repeated use.

```rust
let mut muscle = MuscleMemory::new();
muscle.learn("deploy", "ops", 100);   // Learn a routine costing 100 tokens
muscle.use_routine("deploy");         // Cost: 95  (5% decay)
muscle.use_routine("deploy");         // Cost: 90  (10% decay)
muscle.use_routine("deploy");         // Cost: 85  (15% decay)
// ... 20 total uses → cost reaches 0
muscle.is_free("deploy");             // true
```

**Decay formula:**
```
cost = original × max(0, 1 - uses × 0.05)
```

**Level milestones:**
- Level 3 → cost is halved (after decay)
- Level 5 → cost is 0 (free)

| Method | Description |
|--------|-------------|
| `new()` | Empty muscle memory. |
| `learn(name, domain, cost)` | Register a new routine. |
| `use_routine(name) → u64` | Use a routine, returns cost after decay. |
| `routine_cost(name) → u64` | Current cost (0 if not found). |
| `is_free(name) → bool` | Whether the routine costs 0. |
| `routines_report() → String` | Human-readable status of all routines. |

### TokenEconomy

The top-level system combining ledger, savings, and muscle memory.

```rust
let mut eco = TokenEconomy::new("agent-1", 10_000);

eco.learn_routine("build", "ci", 200);
eco.promote();  // Level 1 → 2

let result = eco.execute_task("build", "ci", 200);
// result.base_cost = 200
// result.actual_cost = depends on routine + discount
// result.routine_used = Some("build")
// result.discount_applied = fraction saved

eco.is_efficient();       // true if tokens/task is decreasing over time
eco.economy_report();     // Full human-readable report
eco.use_muscle_memory("build");  // Direct routine use
```

| Method | Description |
|--------|-------------|
| `new(agent_id, budget)` | Create economy with agent ID and token budget. |
| `execute_task(task, domain, base_tokens) → TaskTokenResult` | Execute task with routine matching + discount applied. |
| `learn_routine(name, domain, cost)` | Register a routine in muscle memory. |
| `use_muscle_memory(name) → u64` | Directly use a routine. |
| `promote()` | Level up: increment abstraction level (max 5) and all routine levels. |
| `is_efficient() → bool` | Whether tokens/task is decreasing over time. |
| `economy_report() → String` | Full status report. |

### TaskTokenResult

```rust
pub struct TaskTokenResult {
    pub task: String,
    pub base_cost: u64,
    pub actual_cost: u64,
    pub savings: u64,
    pub discount_applied: f64,
    pub routine_used: Option<String>,
}
```

---

## How It Works

### Task Execution Flow

```
execute_task("deploy", "ops", 500)
    │
    ├─ 1. Check muscle memory for matching routine (exact name or domain prefix)
    │      → If found, use decayed cost instead of base cost
    │
    ├─ 2. Apply abstraction discount based on current level
    │      → Level 2: 20% off, Level 5: 75% off
    │
    ├─ 3. Record savings in AbstractionSavings
    │
    └─ 4. Spend from ledger, return TaskTokenResult
```

### Promotion

```rust
economy.promote();
```

- Increments abstraction level (1 → 2 → ... → 5, capped).
- Increments *every* routine's level (1 → 2 → ... → 5, capped).
- At routine level 3: half cost. At level 5: free.

### Routine Matching

When executing a task, the economy checks if any routine matches:
- Exact name match: `"deploy"` matches `"deploy"`.
- Prefix match: `"deploy:staging"` matches routine `"deploy"`.

---

## The Math

**Abstraction Discount:**
```
actual = base × (1 - discount(level))
where discount(level) = {0, 0.20, 0.40, 0.60, 0.75} for levels {1, 2, 3, 4, 5+}
```

**Muscle Memory Decay:**
```
cost_decay = original × max(0, 1 - uses × 0.05)
```
After 20 uses, the decay factor reaches 0. With level milestones:
```
if level >= 3: cost = cost_decay / 2
if level >= 5: cost = 0
```

**Effective Cost (during task execution):**
```
effective = min(routine_cost, base_cost)    // if routine found
effective = effective × (1 - discount)       // apply abstraction discount
```

**Budget Constraint:**
```
spend(tokens) succeeds iff used + reserved + tokens ≤ total
```

---

## Tests

39 unit tests covering:

- TokenBudget: new, spend (ok/over), reserve/release, utilization, zero-total edge case
- TokenTransaction: construction, auto-ID generation
- TokenLedger: spend, spend_for_task, category filtering, breakdown, efficiency report, tick
- AbstractionSavings: recording, discount schedule, apply_discount, savings_report
- MuscleMemory: learn, decay progression (95→90→85), level milestones (3=half, 5=free), unknown routine
- TokenEconomy: full lifecycle, execute with/without routine, promotion, efficiency tracking, full serialization round-trip

Run with:
```bash
cargo test
```

---

## License

MIT
