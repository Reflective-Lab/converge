//! Reconciliation Loop — exact matching over noisy ledger data.
//!
//! This is financial-flavored without becoming business theater:
//!
//! - two noisy ledgers enter as data
//! - a candidate scorer builds a cost surface for possible matches
//! - Hungarian assignment from `converge-optimization` finds the exact
//!   one-to-one reconciliation with explicit unmatched slots
//! - a residue summary explains what still needs human review

use std::collections::{BTreeMap, BTreeSet, HashSet};

use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, ConvergeResult, Engine, Fact, Suggestor,
};
use converge_optimization::assignment::{AssignmentProblem, solve as solve_assignment};
use serde::{Deserialize, Serialize};

const SEED_ID: &str = "reconciliation-seed:default";
const MATRIX_ID: &str = "candidate-matrix:default";
const RESULT_ID: &str = "reconciliation-result:default";
const SUMMARY_ID: &str = "reconciliation-summary:default";

const SEED_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Seeds];
const EVALUATION_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Evaluations];
const STRATEGY_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Strategies];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LedgerTransaction {
    id: String,
    posted_day: i32,
    amount_cents: i64,
    counterparty: String,
    reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReconciliationSeed {
    left_label: String,
    right_label: String,
    amount_tolerance_cents: i64,
    max_day_drift: i32,
    unmatched_penalty: i64,
    left: Vec<LedgerTransaction>,
    right: Vec<LedgerTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CandidateScore {
    left_id: String,
    right_id: String,
    cost: i64,
    amount_delta: i64,
    day_delta: i32,
    token_overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CandidateMatrix {
    left_ids: Vec<String>,
    right_ids: Vec<String>,
    unmatched_penalty: i64,
    scores: Vec<CandidateScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchedPair {
    left_id: String,
    right_id: String,
    cost: i64,
    amount_delta: i64,
    day_delta: i32,
    token_overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReconciliationResult {
    matched: Vec<MatchedPair>,
    unmatched_left: Vec<String>,
    unmatched_right: Vec<String>,
    total_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReconciliationSummary {
    matched_count: usize,
    unmatched_left: Vec<String>,
    unmatched_right: Vec<String>,
    suspicious_matches: Vec<String>,
    total_cost: i64,
}

struct CandidateScorerSuggestor;

#[async_trait::async_trait]
impl Suggestor for CandidateScorerSuggestor {
    fn name(&self) -> &str {
        "candidate-scorer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &SEED_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        seed(ctx).is_some() && !fact_exists(ctx, ContextKey::Evaluations, MATRIX_ID)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(seed) = seed(ctx) else {
            return AgentEffect::empty();
        };

        let matrix = build_candidate_matrix(&seed);
        AgentEffect::with_proposal(
            converge_kernel::ProposedFact::new(
                ContextKey::Evaluations,
                MATRIX_ID,
                serde_json::to_string(&matrix).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(0.9),
        )
    }
}

struct ExactAssignmentSuggestor;

#[async_trait::async_trait]
impl Suggestor for ExactAssignmentSuggestor {
    fn name(&self) -> &str {
        "exact-assignment"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &EVALUATION_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        candidate_matrix(ctx).is_some() && !fact_exists(ctx, ContextKey::Strategies, RESULT_ID)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(matrix) = candidate_matrix(ctx) else {
            return AgentEffect::empty();
        };

        let result = reconcile_exact(&matrix);
        AgentEffect::with_proposal(
            converge_kernel::ProposedFact::new(
                ContextKey::Strategies,
                RESULT_ID,
                serde_json::to_string(&result).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(0.94),
        )
    }
}

struct ResidueSummarySuggestor;

#[async_trait::async_trait]
impl Suggestor for ResidueSummarySuggestor {
    fn name(&self) -> &str {
        "residue-summary"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &STRATEGY_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        reconciliation_result(ctx).is_some()
            && !fact_exists(ctx, ContextKey::Diagnostic, SUMMARY_ID)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(result) = reconciliation_result(ctx) else {
            return AgentEffect::empty();
        };

        let suspicious_matches = result
            .matched
            .iter()
            .filter(|pair| pair.cost >= 160)
            .map(|pair| format!("{}<->{}", pair.left_id, pair.right_id))
            .collect();

        let summary = ReconciliationSummary {
            matched_count: result.matched.len(),
            unmatched_left: result.unmatched_left.clone(),
            unmatched_right: result.unmatched_right.clone(),
            suspicious_matches,
            total_cost: result.total_cost,
        };

        AgentEffect::with_proposal(
            converge_kernel::ProposedFact::new(
                ContextKey::Diagnostic,
                SUMMARY_ID,
                serde_json::to_string(&summary).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(0.96),
        )
    }
}

#[tokio::main]
async fn main() {
    println!("=== Reconciliation Loop ===\n");

    let result = run_loop(default_seed())
        .await
        .expect("reconciliation should converge");
    let summary = reconciliation_summary(&result.context).expect("summary should exist");
    let recon = reconciliation_result(&result.context).expect("result should exist");

    println!(
        "Converged: {} (cycles: {}, stop: {:?})",
        result.converged, result.cycles, result.stop_reason
    );
    println!("Matched pairs: {}", summary.matched_count);
    println!("Unmatched left: {}", summary.unmatched_left.join(", "));
    println!("Unmatched right: {}", summary.unmatched_right.join(", "));
    println!("Total cost: {}\n", summary.total_cost);

    println!("Exact matches:");
    for pair in &recon.matched {
        println!(
            "  {} <-> {} (cost={}, amount_delta={}, day_delta={}, overlap={})",
            pair.left_id,
            pair.right_id,
            pair.cost,
            pair.amount_delta,
            pair.day_delta,
            pair.token_overlap
        );
    }
    println!();

    print_section("Evaluations", result.context.get(ContextKey::Evaluations));
    print_section("Strategies", result.context.get(ContextKey::Strategies));
    print_section("Diagnostic", result.context.get(ContextKey::Diagnostic));
}

fn build_engine() -> Engine {
    let mut engine = Engine::with_budget(Budget {
        max_cycles: 8,
        max_facts: 64,
    });
    engine.register_suggestor(CandidateScorerSuggestor);
    engine.register_suggestor(ExactAssignmentSuggestor);
    engine.register_suggestor(ResidueSummarySuggestor);
    engine
}

fn seed_context(seed: &ReconciliationSeed) -> ContextState {
    let mut context = ContextState::new();
    context
        .add_input_with_provenance(
            ContextKey::Seeds,
            SEED_ID,
            serde_json::to_string(seed).unwrap_or_default(),
            "example:reconciliation-loop",
        )
        .expect("should stage seed");
    context
}

async fn run_loop(
    seed: ReconciliationSeed,
) -> Result<ConvergeResult, converge_kernel::ConvergeError> {
    let mut engine = build_engine();
    engine.run(seed_context(&seed)).await
}

fn default_seed() -> ReconciliationSeed {
    ReconciliationSeed {
        left_label: "source-ledger".to_string(),
        right_label: "settlement-ledger".to_string(),
        amount_tolerance_cents: 120,
        max_day_drift: 2,
        unmatched_penalty: 260,
        left: vec![
            txn("l-hosting", 10, 120_000, "acme cloud", "inv-1001"),
            txn("l-coffee", 10, 75_500, "coffee beans", "po-77"),
            txn("l-refund", 12, -18_000, "customer refund", "rf-200"),
            txn("l-payroll", 15, 499_900, "payroll batch", "pay-09"),
            txn("l-payout", 16, 245_000, "stripe payout", "st-884"),
            txn("l-fx", 17, 1_999, "fx spread", "fx-spot"),
        ],
        right: vec![
            txn("r-hosting", 11, 120_000, "acme host", "invoice 1001"),
            txn("r-coffee", 10, 75_480, "beans coffee", "po77"),
            txn("r-refund", 14, -18_000, "refund customer", "rf200"),
            txn("r-payroll", 15, 499_900, "payroll september", "pay09"),
            txn("r-payout", 16, 245_000, "stripe payout", "884"),
            txn("r-fx", 17, 1_999, "fx spread adjustment", "spot"),
            txn("r-bank-fee", 18, 3_500, "bank fee", "svc"),
        ],
    }
}

fn txn(
    id: &str,
    posted_day: i32,
    amount_cents: i64,
    counterparty: &str,
    reference: &str,
) -> LedgerTransaction {
    LedgerTransaction {
        id: id.to_string(),
        posted_day,
        amount_cents,
        counterparty: counterparty.to_string(),
        reference: reference.to_string(),
    }
}

fn seed(ctx: &dyn Context) -> Option<ReconciliationSeed> {
    ctx.get(ContextKey::Seeds)
        .iter()
        .find(|fact| fact.id == SEED_ID)
        .and_then(|fact| serde_json::from_str(&fact.content).ok())
}

fn candidate_matrix(ctx: &dyn Context) -> Option<CandidateMatrix> {
    ctx.get(ContextKey::Evaluations)
        .iter()
        .find(|fact| fact.id == MATRIX_ID)
        .and_then(|fact| serde_json::from_str(&fact.content).ok())
}

fn reconciliation_result(ctx: &dyn Context) -> Option<ReconciliationResult> {
    ctx.get(ContextKey::Strategies)
        .iter()
        .find(|fact| fact.id == RESULT_ID)
        .and_then(|fact| serde_json::from_str(&fact.content).ok())
}

fn reconciliation_summary(ctx: &dyn Context) -> Option<ReconciliationSummary> {
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .find(|fact| fact.id == SUMMARY_ID)
        .and_then(|fact| serde_json::from_str(&fact.content).ok())
}

fn fact_exists(ctx: &dyn Context, key: ContextKey, id: &str) -> bool {
    ctx.get(key).iter().any(|fact| fact.id == id)
}

fn build_candidate_matrix(seed: &ReconciliationSeed) -> CandidateMatrix {
    let mut scores = Vec::new();
    for left in &seed.left {
        for right in &seed.right {
            scores.push(score_pair(left, right, seed));
        }
    }

    CandidateMatrix {
        left_ids: seed.left.iter().map(|txn| txn.id.clone()).collect(),
        right_ids: seed.right.iter().map(|txn| txn.id.clone()).collect(),
        unmatched_penalty: seed.unmatched_penalty,
        scores,
    }
}

fn score_pair(
    left: &LedgerTransaction,
    right: &LedgerTransaction,
    seed: &ReconciliationSeed,
) -> CandidateScore {
    let amount_delta = (left.amount_cents - right.amount_cents).abs();
    let day_delta = (left.posted_day - right.posted_day).abs();
    let token_overlap = token_overlap(left, right);

    let mut cost = amount_delta.min(seed.amount_tolerance_cents * 4);
    cost += i64::from(day_delta.min(seed.max_day_drift * 4)) * 35;

    let description_penalty = match token_overlap {
        0 => 180,
        1 => 60,
        _ => 0,
    };
    cost += description_penalty;

    if amount_delta > seed.amount_tolerance_cents {
        cost += 220;
    }
    if day_delta > seed.max_day_drift {
        cost += 120;
    }

    CandidateScore {
        left_id: left.id.clone(),
        right_id: right.id.clone(),
        cost,
        amount_delta,
        day_delta,
        token_overlap,
    }
}

fn token_overlap(left: &LedgerTransaction, right: &LedgerTransaction) -> usize {
    let left_tokens: HashSet<_> =
        normalized_tokens(&format!("{} {}", left.counterparty, left.reference))
            .into_iter()
            .collect();
    let right_tokens: HashSet<_> =
        normalized_tokens(&format!("{} {}", right.counterparty, right.reference))
            .into_iter()
            .collect();

    left_tokens.intersection(&right_tokens).count()
}

fn normalized_tokens(input: &str) -> Vec<String> {
    let cleaned: String = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    cleaned
        .split_whitespace()
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn reconcile_exact(matrix: &CandidateMatrix) -> ReconciliationResult {
    let left_count = matrix.left_ids.len();
    let right_count = matrix.right_ids.len();
    let total_tasks = right_count + left_count;

    let mut lookup = BTreeMap::new();
    for score in &matrix.scores {
        lookup.insert(
            (score.left_id.clone(), score.right_id.clone()),
            score.clone(),
        );
    }

    let mut costs = vec![vec![matrix.unmatched_penalty + 500; total_tasks]; left_count];
    for (left_idx, left_id) in matrix.left_ids.iter().enumerate() {
        for (right_idx, right_id) in matrix.right_ids.iter().enumerate() {
            let score = lookup
                .get(&(left_id.clone(), right_id.clone()))
                .expect("score must exist");
            costs[left_idx][right_idx] = score.cost;
        }
        for dummy_idx in 0..left_count {
            costs[left_idx][right_count + dummy_idx] = if dummy_idx == left_idx {
                matrix.unmatched_penalty
            } else {
                matrix.unmatched_penalty + 500
            };
        }
    }

    let problem = AssignmentProblem::from_costs(costs);
    let solution = solve_assignment(&problem).expect("assignment should solve");

    let mut matched = Vec::new();
    let mut unmatched_left = Vec::new();
    let mut matched_right_ids = BTreeSet::new();

    for (left_idx, task_idx) in solution.iter() {
        let left_id = &matrix.left_ids[left_idx];
        if task_idx < right_count {
            let right_id = &matrix.right_ids[task_idx];
            let score = lookup
                .get(&(left_id.clone(), right_id.clone()))
                .expect("score must exist");
            if score.cost < matrix.unmatched_penalty {
                matched.push(MatchedPair {
                    left_id: left_id.clone(),
                    right_id: right_id.clone(),
                    cost: score.cost,
                    amount_delta: score.amount_delta,
                    day_delta: score.day_delta,
                    token_overlap: score.token_overlap,
                });
                matched_right_ids.insert(right_id.clone());
                continue;
            }
        }
        unmatched_left.push(left_id.clone());
    }

    matched.sort_by(|left, right| left.left_id.cmp(&right.left_id));
    unmatched_left.sort();

    let unmatched_right: Vec<_> = matrix
        .right_ids
        .iter()
        .filter(|right_id| !matched_right_ids.contains(*right_id))
        .cloned()
        .collect();

    ReconciliationResult {
        matched,
        unmatched_left,
        unmatched_right,
        total_cost: solution.total_cost,
    }
}

fn print_section(title: &str, facts: &[Fact]) {
    println!("{title}:");
    if facts.is_empty() {
        println!("  (none)\n");
        return;
    }

    for fact in facts {
        let preview = if fact.content.len() > 120 {
            format!("{}...", &fact.content[..120])
        } else {
            fact.content.clone()
        };
        println!("  {} ({preview})", fact.id);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_seed_reconciles_expected_pairs_and_leaves_fee_unmatched() {
        let result = run_loop(default_seed()).await.expect("should converge");
        let recon = reconciliation_result(&result.context).expect("result");
        let summary = reconciliation_summary(&result.context).expect("summary");

        let matched_pairs: Vec<_> = recon
            .matched
            .iter()
            .map(|pair| (pair.left_id.as_str(), pair.right_id.as_str()))
            .collect();

        assert_eq!(
            matched_pairs,
            vec![
                ("l-coffee", "r-coffee"),
                ("l-fx", "r-fx"),
                ("l-hosting", "r-hosting"),
                ("l-payout", "r-payout"),
                ("l-payroll", "r-payroll"),
                ("l-refund", "r-refund"),
            ]
        );
        assert!(summary.unmatched_left.is_empty());
        assert_eq!(summary.unmatched_right, vec!["r-bank-fee"]);
    }

    #[tokio::test]
    async fn stricter_threshold_pushes_more_items_to_residue() {
        let mut seed = default_seed();
        seed.amount_tolerance_cents = 0;
        seed.max_day_drift = 0;
        seed.unmatched_penalty = 120;

        let result = run_loop(seed).await.expect("should converge");
        let summary = reconciliation_summary(&result.context).expect("summary");

        assert!(summary.unmatched_right.len() >= 2);
        assert!(!summary.unmatched_left.is_empty());
    }

    #[tokio::test]
    async fn rerun_is_idempotent() {
        let first = run_loop(default_seed()).await.expect("first run");
        let mut engine = build_engine();
        let second = engine
            .run(first.context.clone())
            .await
            .expect("rerun should converge");

        assert_eq!(
            first.context.get(ContextKey::Evaluations).len(),
            second.context.get(ContextKey::Evaluations).len()
        );
        assert_eq!(
            first.context.get(ContextKey::Strategies).len(),
            second.context.get(ContextKey::Strategies).len()
        );
        assert_eq!(
            first.context.get(ContextKey::Diagnostic).len(),
            second.context.get(ContextKey::Diagnostic).len()
        );
    }

    #[test]
    fn exact_assignment_prefers_global_minimum_over_local_greed() {
        let seed = ReconciliationSeed {
            left_label: "left".to_string(),
            right_label: "right".to_string(),
            amount_tolerance_cents: 100,
            max_day_drift: 2,
            unmatched_penalty: 200,
            left: vec![
                txn("l-a", 1, 10_000, "atlas core", "a"),
                txn("l-b", 1, 10_040, "beacon ops", "b"),
            ],
            right: vec![
                txn("r-a", 1, 10_000, "beacon ops", "a"),
                txn("r-b", 1, 10_040, "atlas core", "b"),
            ],
        };

        let matrix = build_candidate_matrix(&seed);
        let result = reconcile_exact(&matrix);
        let matched_pairs: Vec<_> = result
            .matched
            .iter()
            .map(|pair| (pair.left_id.as_str(), pair.right_id.as_str()))
            .collect();

        assert_eq!(matched_pairs, vec![("l-a", "r-b"), ("l-b", "r-a")]);
    }
}
