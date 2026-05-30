//! # lau-token-economy
//!
//! Token budget system — as the agent becomes a journeyman of its shell,
//! it uses fewer tokens for the same tasks. Not by compression, but by
//! abstraction: the journeyman doesn't think about how to hold the hammer.
//!
//! Token usage DECREASES as the agent gains experience in a domain.
//! Level 1: verbose instructions, full context.
//! Level 5: terse commands, compiled routines, muscle memory.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TokenBudget
// ---------------------------------------------------------------------------

/// The token budget for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub total: u64,
    pub used: u64,
    pub reserved: u64,
}

impl TokenBudget {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            used: 0,
            reserved: 0,
        }
    }

    /// Spend `tokens` from the budget. Returns `false` if it would exceed
    /// the total (including reserved amount).
    pub fn spend(&mut self, tokens: u64) -> bool {
        let available = self.total.saturating_sub(self.used + self.reserved);
        if tokens > available {
            return false;
        }
        self.used += tokens;
        true
    }

    /// Reserve tokens for future use. Returns `false` if insufficient budget
    /// remains.
    pub fn reserve(&mut self, tokens: u64) -> bool {
        let available = self.total.saturating_sub(self.used + self.reserved);
        if tokens > available {
            return false;
        }
        self.reserved += tokens;
        true
    }

    /// Release previously-reserved tokens back into the pool.
    pub fn release(&mut self, tokens: u64) {
        self.reserved = self.reserved.saturating_sub(tokens);
    }

    pub fn remaining(&self) -> u64 {
        self.total.saturating_sub(self.used + self.reserved)
    }

    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.used as f64) / (self.total as f64)
    }
}

// ---------------------------------------------------------------------------
// TokenCategory
// ---------------------------------------------------------------------------

/// Categories of token expenditure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenCategory {
    SystemPrompt,
    ContextLoad,
    ToolCall,
    ToolResult,
    ModelInference,
    MemoryRecall,
    Compilation,
    Abstraction,
    Delegation,
}

impl std::fmt::Display for TokenCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemPrompt => write!(f, "SystemPrompt"),
            Self::ContextLoad => write!(f, "ContextLoad"),
            Self::ToolCall => write!(f, "ToolCall"),
            Self::ToolResult => write!(f, "ToolResult"),
            Self::ModelInference => write!(f, "ModelInference"),
            Self::MemoryRecall => write!(f, "MemoryRecall"),
            Self::Compilation => write!(f, "Compilation"),
            Self::Abstraction => write!(f, "Abstraction"),
            Self::Delegation => write!(f, "Delegation"),
        }
    }
}

// ---------------------------------------------------------------------------
// TokenTransaction
// ---------------------------------------------------------------------------

/// A record of token usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransaction {
    pub id: String,
    pub tick: u64,
    pub tokens: u64,
    pub category: TokenCategory,
    pub description: String,
}

impl TokenTransaction {
    pub fn new(tokens: u64, category: TokenCategory, description: &str, tick: u64) -> Self {
        let id = uuid_v4();
        Self {
            id,
            tick,
            tokens,
            category,
            description: description.to_string(),
        }
    }
}

/// Simple v4 UUID-like identifier (no external dependency).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ts = now.as_nanos();
    let r1: u64 = (((ts >> 32) ^ (ts & 0xFFFF_FFFF)) & 0xFFFF_FFFF) as u64;
    let r2: u64 = (((ts >> 16) ^ (ts & 0xFFFF)) & 0xFFFF) as u64;
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (ts & 0xFFFF_FFFF) as u32,
        (r1 >> 16) as u16,
        (r1 & 0x0FFF) as u16,
        (0x8000 | (r2 & 0x3FFF)) as u16,
        ts & 0xFFFF_FFFF_FFFF
    )
}

// ---------------------------------------------------------------------------
// TokenLedger
// ---------------------------------------------------------------------------

/// Double-entry bookkeeping for tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLedger {
    pub transactions: Vec<TokenTransaction>,
    pub budget: TokenBudget,
    pub tick: u64,
}

impl TokenLedger {
    pub fn new(budget_total: u64) -> Self {
        Self {
            transactions: Vec::new(),
            budget: TokenBudget::new(budget_total),
            tick: 0,
        }
    }

    /// Record a token spend. Returns `false` if over budget.
    pub fn spend(&mut self, tokens: u64, category: TokenCategory, description: &str) -> bool {
        if !self.budget.spend(tokens) {
            return false;
        }
        let tx = TokenTransaction::new(tokens, category, description, self.tick);
        self.transactions.push(tx);
        true
    }

    /// Spend tokens attributed to a named task.
    pub fn spend_for_task(&mut self, task: &str, tokens: u64) -> bool {
        self.spend(tokens, TokenCategory::Compilation, task)
    }

    pub fn transactions_by_category(
        &self,
        category: &TokenCategory,
    ) -> Vec<&TokenTransaction> {
        self.transactions
            .iter()
            .filter(|tx| tx.category == *category)
            .collect()
    }

    pub fn total_by_category(&self, category: &TokenCategory) -> u64 {
        self.transactions
            .iter()
            .filter(|tx| tx.category == *category)
            .map(|tx| tx.tokens)
            .sum()
    }

    pub fn category_breakdown(&self) -> HashMap<String, u64> {
        let mut map = HashMap::new();
        for tx in &self.transactions {
            *map.entry(tx.category.to_string()).or_insert(0) += tx.tokens;
        }
        map
    }

    pub fn efficiency_report(&self) -> TokenEfficiencyReport {
        let total_tokens: u64 = self.transactions.iter().map(|tx| tx.tokens).sum();
        let task_count = self
            .transactions
            .iter()
            .filter(|tx| tx.category == TokenCategory::Compilation)
            .count()
            .max(1);
        let tokens_per_task = total_tokens as f64 / task_count as f64;

        // Find most expensive category
        let breakdown = self.category_breakdown();
        let most_expensive_category = breakdown
            .iter()
            .max_by_key(|&(_, v)| v)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "None".to_string());

        // Efficiency trend: tokens/task over time (per tick)
        let max_tick = self.tick.max(1);
        let efficiency_trend: Vec<f64> = (0..=max_tick)
            .map(|t| {
                let txns: Vec<&TokenTransaction> = self
                    .transactions
                    .iter()
                    .filter(|tx| tx.tick <= t)
                    .collect();
                if txns.is_empty() {
                    return 0.0;
                }
                let subtotal: u64 = txns.iter().map(|tx| tx.tokens).sum();
                subtotal as f64 / (txns.len().max(1) as f64)
            })
            .collect();

        TokenEfficiencyReport {
            total_tokens,
            tokens_per_task,
            tokens_saved_by_abstraction: 0, // filled in by TokenEconomy
            most_expensive_category,
            efficiency_trend,
            journeyman_discount: 0.0,
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
    }
}

// ---------------------------------------------------------------------------
// TokenEfficiencyReport
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEfficiencyReport {
    pub total_tokens: u64,
    pub tokens_per_task: f64,
    pub tokens_saved_by_abstraction: u64,
    pub most_expensive_category: String,
    pub efficiency_trend: Vec<f64>,
    pub journeyman_discount: f64,
}

// ---------------------------------------------------------------------------
// AbstractionSavings
// ---------------------------------------------------------------------------

/// Tracks how much abstraction saves over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionSavings {
    pub abstraction_level: u32,
    pub base_cost: u64,
    pub actual_cost: u64,
    pub savings: u64,
    pub total_savings: u64,
    /// (level, cumulative_savings)
    pub savings_history: Vec<(u32, u64)>,
}

impl AbstractionSavings {
    pub fn new() -> Self {
        Self {
            abstraction_level: 1,
            base_cost: 0,
            actual_cost: 0,
            savings: 0,
            total_savings: 0,
            savings_history: Vec::new(),
        }
    }

    /// Record a transaction with abstraction savings.
    pub fn record(&mut self, level: u32, base_cost: u64, actual_cost: u64) {
        self.abstraction_level = level;
        self.base_cost = base_cost;
        self.actual_cost = actual_cost;
        self.savings = base_cost.saturating_sub(actual_cost);
        self.total_savings += self.savings;
        self.savings_history.push((level, self.total_savings));
    }

    /// Level 1: 0%, Level 2: 20%, Level 3: 40%, Level 4: 60%, Level 5: 75%
    pub fn discount_for_level(&self, level: u32) -> f64 {
        match level {
            1 => 0.0,
            2 => 0.20,
            3 => 0.40,
            4 => 0.60,
            5 => 0.75,
            _ if level > 5 => 0.75,
            _ => 0.0,
        }
    }

    /// Apply journeyman discount to base tokens.
    pub fn apply_discount(&self, level: u32, base_tokens: u64) -> u64 {
        let discount = self.discount_for_level(level);
        let saved = (base_tokens as f64 * discount) as u64;
        base_tokens.saturating_sub(saved)
    }

    pub fn savings_report(&self) -> String {
        format!(
            "Abstraction Level: {}\n\
             Total Savings: {} tokens\n\
             Current discount: {:.0}%\n\
             Savings recorded: {}",
            self.abstraction_level,
            self.total_savings,
            self.discount_for_level(self.abstraction_level) * 100.0,
            self.savings_history.len()
        )
    }
}

impl Default for AbstractionSavings {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MuscleMemory
// ---------------------------------------------------------------------------

/// A compiled routine in muscle memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    pub name: String,
    pub domain: String,
    /// What the task used to cost
    pub tokens_before: u64,
    /// What it costs now
    pub tokens_after: u64,
    pub times_used: u32,
    pub level: u32,
}

/// Routines that cost zero (or near-zero) context tokens.
///
/// Cost decay: starts at `original_cost`, decreases by 5% per use,
/// minimum 0.
///
/// Level milestones:
/// - Level 3 → half cost
/// - Level 5 → free
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuscleMemory {
    pub routines: HashMap<String, Routine>,
}

impl MuscleMemory {
    pub fn new() -> Self {
        Self {
            routines: HashMap::new(),
        }
    }

    /// Add a routine to memory.
    pub fn learn(&mut self, name: &str, domain: &str, original_cost: u64) {
        let level = 1;
        self.routines.insert(
            name.to_string(),
            Routine {
                name: name.to_string(),
                domain: domain.to_string(),
                tokens_before: original_cost,
                tokens_after: original_cost,
                times_used: 0,
                level,
            },
        );
    }

    /// Use a routine. Returns the token cost after decay.
    /// Returns 0 if the routine doesn't exist.
    pub fn use_routine(&mut self, name: &str) -> u64 {
        // Check existence first, then do the update
        let exists = self.routines.contains_key(name);
        if !exists {
            return 0;
        }
        // Increment first so this use counts toward decay
        if let Some(routine) = self.routines.get_mut(name) {
            routine.times_used += 1;
        }
        let cost = self.routine_cost(name);
        if let Some(routine) = self.routines.get_mut(name) {
            routine.tokens_after = cost;
        }
        cost
    }

    /// Current cost of a routine after all discounts and decay.
    pub fn routine_cost(&self, name: &str) -> u64 {
        let Some(routine) = self.routines.get(name) else {
            return 0;
        };

        // Level milestone: Level 5 = free
        if routine.level >= 5 {
            return 0;
        }

        let original = routine.tokens_before;
        // 5% decay per use
        let decay_factor = 1.0 - (routine.times_used as f64 * 0.05);
        let decayed = (original as f64 * decay_factor.max(0.0)).round() as u64;

        // Level 3 = half cost
        if routine.level >= 3 {
            decayed / 2
        } else {
            decayed
        }
    }

    /// Whether a routine is pure muscle memory (cost == 0).
    pub fn is_free(&self, name: &str) -> bool {
        if !self.routines.contains_key(name) {
            return false;
        }
        self.routine_cost(name) == 0
    }

    pub fn routines_report(&self) -> String {
        if self.routines.is_empty() {
            return "No routines learned yet.".to_string();
        }
        let mut lines: Vec<String> = Vec::new();
        for r in self.routines.values() {
            lines.push(format!(
                "{} [{}] | before: {} | after: {} | used: {} | level: {}",
                r.name, r.domain, r.tokens_before, r.tokens_after, r.times_used, r.level
            ));
        }
        lines.join("\n")
    }
}

impl Default for MuscleMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TokenEconomy
// ---------------------------------------------------------------------------

/// Result of executing a task through the economy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTokenResult {
    pub task: String,
    pub base_cost: u64,
    pub actual_cost: u64,
    pub savings: u64,
    pub discount_applied: f64,
    pub routine_used: Option<String>,
}

/// THE token economy management system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEconomy {
    pub ledger: TokenLedger,
    pub savings: AbstractionSavings,
    pub muscle: MuscleMemory,
    pub agent_id: String,
    pub session_count: u32,
}

impl TokenEconomy {
    pub fn new(agent_id: &str, budget: u64) -> Self {
        Self {
            ledger: TokenLedger::new(budget),
            savings: AbstractionSavings::new(),
            muscle: MuscleMemory::new(),
            agent_id: agent_id.to_string(),
            session_count: 0,
        }
    }

    /// Execute a task, returning actual cost after discounts and muscle memory.
    pub fn execute_task(
        &mut self,
        task: &str,
        _domain: &str,
        base_tokens: u64,
    ) -> TaskTokenResult {
        let mut actual_cost = base_tokens;
        let mut routine_used: Option<String> = None;

        // Check if a routine exists for this task by exact name or by domain prefix
        let matched_routine: Option<String> = self
            .muscle
            .routines
            .keys()
            .find(|r| **r == task || task.starts_with(&format!("{}:", r)))
            .cloned();

        if let Some(ref name) = matched_routine {
            let cost = self.muscle.use_routine(name);
            // Only apply routine cost if it's less than base
            if cost < base_tokens {
                actual_cost = cost;
                routine_used = Some(name.clone());
            }
        }

        // Apply abstraction discount
        let level = self.savings.abstraction_level;
        let discounted = self.savings.apply_discount(level, actual_cost);
        if discounted < actual_cost {
            actual_cost = discounted;
        }

        // Record savings
        let savings = base_tokens.saturating_sub(actual_cost);
        self.savings
            .record(level, base_tokens, actual_cost);

        // Spend from ledger
        self.ledger.spend(
            actual_cost,
            TokenCategory::Compilation,
            task,
        );

        let discount_applied = if base_tokens == 0 {
            0.0
        } else {
            savings as f64 / base_tokens as f64
        };

        TaskTokenResult {
            task: task.to_string(),
            base_cost: base_tokens,
            actual_cost,
            savings,
            discount_applied,
            routine_used,
        }
    }

    /// Use a learned routine at near-zero cost.
    pub fn use_muscle_memory(&mut self, routine: &str) -> u64 {
        self.muscle.use_routine(routine)
    }

    /// Learn a new routine.
    pub fn learn_routine(&mut self, name: &str, domain: &str, cost: u64) {
        self.muscle.learn(name, domain, cost);
    }

    /// Agent leveled up — discounts increase.
    pub fn promote(&mut self) {
        let next = (self.savings.abstraction_level + 1).min(5);
        self.savings.abstraction_level = next;

        // Also promote all routines
        for routine in self.muscle.routines.values_mut() {
            routine.level = (routine.level + 1).min(5);
        }
    }

    /// Full economy report.
    pub fn economy_report(&self) -> String {
        let eff = self.ledger.efficiency_report();
        format!(
            "=== Token Economy Report ===\n\
             Agent: {agent}\n\
             Sessions: {sessions}\n\
             Budget: {total} / Used: {used} / Reserved: {reserved}\n\
             Utilization: {util:.1}%\n\
             Level: {level} (discount: {disc:.0}%)\n\
             Total abstraction savings: {savings} tokens\n\
             Routines learned: {routines}\n\
             Tokens/task: {tpt:.1}\n\
             Most expensive category: {mec}\n\
             Journeyman discount: {jd:.1}%",
            agent = self.agent_id,
            sessions = self.session_count,
            total = self.ledger.budget.total,
            used = self.ledger.budget.used,
            reserved = self.ledger.budget.reserved,
            util = self.ledger.budget.utilization() * 100.0,
            level = self.savings.abstraction_level,
            disc = self.savings.discount_for_level(self.savings.abstraction_level) * 100.0,
            savings = self.savings.total_savings,
            routines = self.muscle.routines.len(),
            tpt = eff.tokens_per_task,
            mec = eff.most_expensive_category,
            jd = eff.journeyman_discount * 100.0,
        )
    }

    /// Returns `true` if tokens/task is decreasing over time.
    pub fn is_efficient(&self) -> bool {
        let trend = self.ledger.efficiency_report().efficiency_trend;
        if trend.len() < 3 {
            return false;
        }
        // Check if trend is generally decreasing
        trend.windows(2).all(|w| w[0] >= w[1])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- TokenBudget ----

    #[test]
    fn test_budget_new() {
        let b = TokenBudget::new(1000);
        assert_eq!(b.total, 1000);
        assert_eq!(b.used, 0);
        assert_eq!(b.reserved, 0);
    }

    #[test]
    fn test_budget_spend_ok() {
        let mut b = TokenBudget::new(100);
        assert!(b.spend(40));
        assert_eq!(b.used, 40);
        assert_eq!(b.remaining(), 60);
    }

    #[test]
    fn test_budget_spend_over() {
        let mut b = TokenBudget::new(100);
        assert!(b.spend(100));
        assert!(!b.spend(1));
    }

    #[test]
    fn test_budget_reserve_and_release() {
        let mut b = TokenBudget::new(100);
        assert!(b.reserve(30));
        assert_eq!(b.reserved, 30);
        assert_eq!(b.remaining(), 70);
        b.release(10);
        assert_eq!(b.reserved, 20);
        assert_eq!(b.remaining(), 80);
    }

    #[test]
    fn test_budget_reserve_over() {
        let mut b = TokenBudget::new(50);
        assert!(!b.reserve(100));
        assert_eq!(b.reserved, 0);
    }

    #[test]
    fn test_budget_utilization() {
        let mut b = TokenBudget::new(200);
        assert_eq!(b.utilization(), 0.0);
        b.spend(50);
        assert!((b.utilization() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_budget_utilization_zero_total() {
        let b = TokenBudget::new(0);
        assert_eq!(b.utilization(), 0.0);
    }

    // ---- TokenTransaction ----

    #[test]
    fn test_transaction_new() {
        let tx = TokenTransaction::new(42, TokenCategory::ToolCall, "ls -la", 5);
        assert_eq!(tx.tokens, 42);
        assert_eq!(tx.category, TokenCategory::ToolCall);
        assert_eq!(tx.description, "ls -la");
        assert_eq!(tx.tick, 5);
        assert!(!tx.id.is_empty());
    }

    // ---- TokenLedger ----

    #[test]
    fn test_ledger_new() {
        let l = TokenLedger::new(500);
        assert_eq!(l.budget.total, 500);
        assert!(l.transactions.is_empty());
    }

    #[test]
    fn test_ledger_spend_ok() {
        let mut l = TokenLedger::new(100);
        assert!(l.spend(30, TokenCategory::ModelInference, "test inference"));
        assert_eq!(l.transactions.len(), 1);
        assert_eq!(l.budget.used, 30);
    }

    #[test]
    fn test_ledger_spend_over() {
        let mut l = TokenLedger::new(10);
        assert!(!l.spend(20, TokenCategory::ModelInference, "too much"));
    }

    #[test]
    fn test_ledger_spend_for_task() {
        let mut l = TokenLedger::new(100);
        assert!(l.spend_for_task("build", 50));
        assert_eq!(l.transactions[0].category, TokenCategory::Compilation);
        assert_eq!(l.transactions[0].description, "build");
    }

    #[test]
    fn test_ledger_by_category() {
        let mut l = TokenLedger::new(1000);
        l.spend(10, TokenCategory::ToolCall, "tool1");
        l.spend(20, TokenCategory::ToolCall, "tool2");
        l.spend(30, TokenCategory::ContextLoad, "ctx");
        let calls = l.transactions_by_category(&TokenCategory::ToolCall);
        assert_eq!(calls.len(), 2);
        assert_eq!(l.total_by_category(&TokenCategory::ToolCall), 30);
        assert_eq!(l.total_by_category(&TokenCategory::ContextLoad), 30);
    }

    #[test]
    fn test_ledger_category_breakdown() {
        let mut l = TokenLedger::new(1000);
        l.spend(10, TokenCategory::MemoryRecall, "mem1");
        l.spend(20, TokenCategory::MemoryRecall, "mem2");
        l.spend(50, TokenCategory::Delegation, "delegate");
        let map = l.category_breakdown();
        assert_eq!(map.get("MemoryRecall"), Some(&30));
        assert_eq!(map.get("Delegation"), Some(&50));
    }

    #[test]
    fn test_ledger_tick() {
        let mut l = TokenLedger::new(1000);
        assert_eq!(l.tick, 0);
        l.tick();
        assert_eq!(l.tick, 1);
    }

    #[test]
    fn test_ledger_efficiency_report() {
        let mut l = TokenLedger::new(1000);
        l.spend_for_task("task1", 100);
        l.tick();
        l.spend_for_task("task2", 50);
        let report = l.efficiency_report();
        assert_eq!(report.total_tokens, 150);
        assert!((report.tokens_per_task - 75.0).abs() < 1e-10);
        assert_eq!(report.most_expensive_category, "Compilation");
        assert!(report.efficiency_trend.len() >= 2);
    }

    // ---- AbstractionSavings ----

    #[test]
    fn test_savings_new() {
        let s = AbstractionSavings::new();
        assert_eq!(s.abstraction_level, 1);
        assert_eq!(s.total_savings, 0);
    }

    #[test]
    fn test_savings_record() {
        let mut s = AbstractionSavings::new();
        s.record(2, 100, 80);
        assert_eq!(s.savings, 20);
        assert_eq!(s.total_savings, 20);
        assert_eq!(s.savings_history.len(), 1);
    }

    #[test]
    fn test_savings_discount_for_level() {
        let s = AbstractionSavings::new();
        assert!((s.discount_for_level(1) - 0.0).abs() < 1e-10);
        assert!((s.discount_for_level(2) - 0.20).abs() < 1e-10);
        assert!((s.discount_for_level(3) - 0.40).abs() < 1e-10);
        assert!((s.discount_for_level(4) - 0.60).abs() < 1e-10);
        assert!((s.discount_for_level(5) - 0.75).abs() < 1e-10);
        assert!((s.discount_for_level(6) - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_savings_apply_discount() {
        let s = AbstractionSavings::new();
        assert_eq!(s.apply_discount(1, 100), 100);
        assert_eq!(s.apply_discount(2, 100), 80);
        assert_eq!(s.apply_discount(3, 100), 60);
        assert_eq!(s.apply_discount(4, 100), 40);
        assert_eq!(s.apply_discount(5, 100), 25);
    }

    #[test]
    fn test_savings_report() {
        let mut s = AbstractionSavings::new();
        s.record(3, 200, 120);
        let r = s.savings_report();
        assert!(r.contains("Level: 3"));
        assert!(r.contains("80 tokens")); // savings
    }

    // ---- MuscleMemory ----

    #[test]
    fn test_muscle_new() {
        let m = MuscleMemory::new();
        assert!(m.routines.is_empty());
    }

    #[test]
    fn test_muscle_learn() {
        let mut m = MuscleMemory::new();
        m.learn("deploy", "devops", 100);
        assert!(m.routines.contains_key("deploy"));
        assert_eq!(m.routines["deploy"].tokens_before, 100);
    }

    #[test]
    fn test_muscle_use_and_decay() {
        let mut m = MuscleMemory::new();
        m.learn("git push", "git", 100);
        let cost1 = m.use_routine("git push");
        assert_eq!(cost1, 95); // 5% decay (100*0.95=95)
        let cost2 = m.use_routine("git push");
        assert_eq!(cost2, 90); // 10% decay (100*0.90=90)
        let cost3 = m.use_routine("git push");
        assert_eq!(cost3, 85); // 15% decay (100*0.85=85)
    }

    #[test]
    fn test_muscle_level_milestones() {
        let mut m = MuscleMemory::new();
        m.learn("build", "ci", 100);
        // Level 1, 20 uses should get to free via decay
        for _ in 0..20 {
            m.use_routine("build");
        }
        assert_eq!(m.routine_cost("build"), 0);
    }

    #[test]
    fn test_muscle_level_three_half_cost() {
        let mut m = MuscleMemory::new();
        m.learn("test", "ci", 100);
        // Manually set level to 3
        m.routines.get_mut("test").unwrap().level = 3;
        // 0 uses → decay doesn't apply, but level 3 = half cost
        assert_eq!(m.routine_cost("test"), 50);
    }

    #[test]
    fn test_muscle_level_five_free() {
        let mut m = MuscleMemory::new();
        m.learn("deploy", "ops", 500);
        m.routines.get_mut("deploy").unwrap().level = 5;
        assert_eq!(m.routine_cost("deploy"), 0);
        assert!(m.is_free("deploy"));
    }

    #[test]
    fn test_muscle_unknown_routine() {
        let m = MuscleMemory::new();
        assert_eq!(m.routine_cost("nonexistent"), 0);
        assert!(!m.is_free("nonexistent"));
    }

    #[test]
    fn test_muscle_routines_report_empty() {
        let m = MuscleMemory::new();
        assert_eq!(m.routines_report(), "No routines learned yet.");
    }

    // ---- TokenEconomy ----

    #[test]
    fn test_economy_new() {
        let e = TokenEconomy::new("agent-1", 10_000);
        assert_eq!(e.agent_id, "agent-1");
        assert_eq!(e.ledger.budget.total, 10_000);
        assert_eq!(e.session_count, 0);
    }

    #[test]
    fn test_economy_execute_task_base() {
        let mut e = TokenEconomy::new("test-agent", 10_000);
        let result = e.execute_task("compile", "dev", 200);
        assert_eq!(result.base_cost, 200);
        assert_eq!(result.actual_cost, 200); // Level 1 = no discount
        assert_eq!(result.savings, 0);
        assert_eq!(result.task, "compile");
        assert!(result.routine_used.is_none());
    }

    #[test]
    fn test_economy_execute_task_with_discount() {
        let mut e = TokenEconomy::new("test-agent", 10_000);
        e.promote(); // level 2 → 20% discount
        let result = e.execute_task("build", "dev", 100);
        assert_eq!(result.actual_cost, 80);
        assert_eq!(result.savings, 20);
        assert!((result.discount_applied - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_economy_execute_task_with_routine() {
        let mut e = TokenEconomy::new("test-agent", 10_000);
        e.learn_routine("deploy", "ops", 500);
        let result = e.execute_task("deploy", "ops", 500);
        assert_eq!(result.routine_used, Some("deploy".to_string()));
        // Level 1, first use → 5% decay = 475
        assert_eq!(result.actual_cost, 475);
    }

    #[test]
    fn test_economy_learn_and_use_muscle() {
        let mut e = TokenEconomy::new("agent-x", 10_000);
        e.learn_routine("git commit", "git", 50);
        let cost = e.use_muscle_memory("git commit");
        // 5% off 50 = 2.5, round(47.5) = 48
        assert_eq!(cost, 48);
    }

    #[test]
    fn test_economy_promote() {
        let mut e = TokenEconomy::new("agent", 10_000);
        assert_eq!(e.savings.abstraction_level, 1);
        e.learn_routine("fmt", "fmt", 100);
        assert_eq!(e.muscle.routines["fmt"].level, 1);

        e.promote();
        assert_eq!(e.savings.abstraction_level, 2);
        assert_eq!(e.muscle.routines["fmt"].level, 2);

        // Promote to max
        for _ in 0..5 {
            e.promote();
        }
        assert_eq!(e.savings.abstraction_level, 5);
        assert_eq!(e.muscle.routines["fmt"].level, 5);
    }

    #[test]
    fn test_economy_economy_report() {
        let mut e = TokenEconomy::new("bob", 5000);
        e.execute_task("task1", "gen", 100);
        e.execute_task("task2", "gen", 50);
        let report = e.economy_report();
        assert!(report.contains("bob"));
        assert!(report.contains("Budget: 5000"));
        assert!(report.contains("Level: 1"));
    }

    #[test]
    fn test_economy_is_efficient_not_enough_data() {
        let e = TokenEconomy::new("t", 1000);
        assert!(!e.is_efficient());
    }

    #[test]
    fn test_economy_is_efficient() {
        let mut e = TokenEconomy::new("t", 10_000);
        e.execute_task("big", "dev", 200);
        e.ledger.tick();
        e.execute_task("med", "dev", 100);
        e.ledger.tick();
        e.execute_task("small", "dev", 50);
        e.ledger.tick();
        assert!(e.is_efficient());
    }

    #[test]
    fn test_economy_serialization_roundtrip() {
        let mut e = TokenEconomy::new("ser-test", 5000);
        e.execute_task("t1", "d1", 100);
        e.learn_routine("r1", "d2", 50);
        e.promote();
        let json = serde_json::to_string(&e).unwrap();
        let deser: TokenEconomy = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.agent_id, "ser-test");
        assert_eq!(deser.savings.abstraction_level, 2);
        assert!(deser.muscle.routines.contains_key("r1"));
        assert_eq!(deser.ledger.budget.used, 100);
    }
}
