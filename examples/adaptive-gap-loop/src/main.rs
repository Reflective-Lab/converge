//! Adaptive Gap Loop — generic example of open-ended convergence.
//!
//! Shows the shape that bounded demos cannot:
//!
//! 1. A seed asks the engine to inspect one artifact.
//! 2. The survey suggestor emits observations about that artifact.
//! 3. The gap suggestor notices referenced artifacts that have not been
//!    explored yet and reopens the loop with new survey requests.
//! 4. The closure suggestor only emits a summary once every discovered gap is
//!    covered.
//!
//! The important point is that the loop length is emergent. A shallow graph may
//! settle in a couple of cycles; a deeper graph may need several rehuddles
//! before the context reaches a fixed point.

use std::collections::BTreeSet;

use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, ConvergeResult, Engine, Fact, Suggestor,
};
use serde::{Deserialize, Serialize};

const ROOT_PREFIX: &str = "artifact-root:";
const REQUEST_PREFIX: &str = "survey-request:";
const SIGNAL_PREFIX: &str = "artifact-signal:";
const GAP_PREFIX: &str = "open-gap:";
const SUMMARY_PREFIX: &str = "closure:";
const UNKNOWN_PREFIX: &str = "unknown-artifact:";

const ROOT_DEPENDENCIES: [ContextKey; 2] = [ContextKey::Seeds, ContextKey::Strategies];
const SIGNAL_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Signals];
const CLOSURE_DEPENDENCIES: [ContextKey; 3] = [
    ContextKey::Seeds,
    ContextKey::Signals,
    ContextKey::Strategies,
];

#[derive(Debug, Clone, Copy)]
struct ArtifactProfile {
    kind: &'static str,
    summary: &'static str,
    depends_on: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SurveyRequest {
    root: String,
    target: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ArtifactSignal {
    root: String,
    target: String,
    kind: String,
    summary: String,
    depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct GapNotice {
    root: String,
    source: String,
    missing: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ClosureSummary {
    root: String,
    surveyed: Vec<String>,
    rehuddle_count: usize,
    pending_requests: Vec<String>,
    open_dependencies: Vec<String>,
}

struct ArtifactSurveySuggestor;

#[async_trait::async_trait]
impl Suggestor for ArtifactSurveySuggestor {
    fn name(&self) -> &str {
        "artifact-survey"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &ROOT_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        pending_surveys(ctx)
            .into_iter()
            .any(|request| !signal_exists(ctx, &request.root, &request.target))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for request in pending_surveys(ctx) {
            if signal_exists(ctx, &request.root, &request.target) {
                continue;
            }

            match artifact_profile(&request.target) {
                Some(profile) => {
                    let signal = ArtifactSignal {
                        root: request.root.clone(),
                        target: request.target.clone(),
                        kind: profile.kind.to_string(),
                        summary: profile.summary.to_string(),
                        depends_on: profile
                            .depends_on
                            .iter()
                            .map(|dependency| (*dependency).to_string())
                            .collect(),
                    };
                    proposals.push(
                        converge_kernel::ProposedFact::new(
                            ContextKey::Signals,
                            signal_id(&request.root, &request.target),
                            serde_json::to_string(&signal).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(0.92),
                    );
                }
                None => {
                    let signal = ArtifactSignal {
                        root: request.root.clone(),
                        target: request.target.clone(),
                        kind: "opaque".to_string(),
                        summary: format!(
                            "no profile registered for {}; treat as an opaque leaf",
                            request.target
                        ),
                        depends_on: Vec::new(),
                    };
                    proposals.push(
                        converge_kernel::ProposedFact::new(
                            ContextKey::Signals,
                            signal_id(&request.root, &request.target),
                            serde_json::to_string(&signal).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(0.7),
                    );
                    proposals.push(
                        converge_kernel::ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("{UNKNOWN_PREFIX}{}:{}", request.root, request.target),
                            format!(
                                "{} had no catalog entry, so the loop treated it as a leaf",
                                request.target
                            ),
                            self.name(),
                        )
                        .with_confidence(0.75),
                    );
                }
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

struct GapHuddleSuggestor;

#[async_trait::async_trait]
impl Suggestor for GapHuddleSuggestor {
    fn name(&self) -> &str {
        "gap-huddle"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &SIGNAL_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        signals(ctx).into_iter().any(|signal| {
            signal.depends_on.iter().any(|dependency| {
                !signal_exists(ctx, &signal.root, dependency)
                    && !request_exists(ctx, &signal.root, dependency)
            })
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for signal in signals(ctx) {
            for dependency in &signal.depends_on {
                if signal_exists(ctx, &signal.root, dependency)
                    || request_exists(ctx, &signal.root, dependency)
                {
                    continue;
                }

                let request = SurveyRequest {
                    root: signal.root.clone(),
                    target: dependency.clone(),
                    reason: format!("{} references {}", signal.target, dependency),
                };
                let gap = GapNotice {
                    root: signal.root.clone(),
                    source: signal.target.clone(),
                    missing: dependency.clone(),
                    message: format!(
                        "{} mentions {}; rehuddle and inspect that artifact next",
                        signal.target, dependency
                    ),
                };

                proposals.push(
                    converge_kernel::ProposedFact::new(
                        ContextKey::Hypotheses,
                        gap_id(&signal.root, &signal.target, dependency),
                        serde_json::to_string(&gap).unwrap_or_default(),
                        self.name(),
                    )
                    .with_confidence(0.83),
                );
                proposals.push(
                    converge_kernel::ProposedFact::new(
                        ContextKey::Strategies,
                        request_id(&signal.root, dependency),
                        serde_json::to_string(&request).unwrap_or_default(),
                        self.name(),
                    )
                    .with_confidence(0.9),
                );
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

struct ClosureSuggestor;

#[async_trait::async_trait]
impl Suggestor for ClosureSuggestor {
    fn name(&self) -> &str {
        "closure"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &CLOSURE_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        roots(ctx)
            .into_iter()
            .any(|root| !summary_exists(ctx, &root) && closure_inputs(ctx, &root).is_some())
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for root in roots(ctx) {
            if summary_exists(ctx, &root) {
                continue;
            }

            let Some(summary) = closure_inputs(ctx, &root) else {
                continue;
            };

            proposals.push(
                converge_kernel::ProposedFact::new(
                    ContextKey::Diagnostic,
                    summary_id(&root),
                    serde_json::to_string(&summary).unwrap_or_default(),
                    self.name(),
                )
                .with_confidence(0.96),
            );
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[tokio::main]
async fn main() {
    println!("=== Adaptive Gap Loop ===\n");

    let result = run_loop("edge-gateway", 20)
        .await
        .expect("adaptive loop should run");
    let summary = parse_summary(&result.context, "edge-gateway").expect("closure summary");

    println!("Converged: {}", result.converged);
    println!("Cycles:    {}", result.cycles);
    println!("Stop:      {:?}\n", result.stop_reason);
    println!("Surveyed artifacts: {}", summary.surveyed.join(", "));
    println!("Rehuddles:         {}", summary.rehuddle_count);
    println!(
        "Pending/open:      {} / {}\n",
        summary.pending_requests.len(),
        summary.open_dependencies.len()
    );

    print_section("Signals", result.context.get(ContextKey::Signals));
    print_section("Hypotheses", result.context.get(ContextKey::Hypotheses));
    print_section("Strategies", result.context.get(ContextKey::Strategies));
    print_section("Diagnostic", result.context.get(ContextKey::Diagnostic));
}

fn build_engine(max_cycles: u32) -> Engine {
    let mut engine = Engine::with_budget(Budget {
        max_cycles,
        max_facts: 256,
    });
    engine.register_suggestor(ArtifactSurveySuggestor);
    engine.register_suggestor(GapHuddleSuggestor);
    engine.register_suggestor(ClosureSuggestor);
    engine
}

fn seed_root(root: &str) -> ContextState {
    let mut context = ContextState::new();
    context
        .add_input_with_provenance(
            ContextKey::Seeds,
            root_id(root),
            root,
            "example:adaptive-gap-loop",
        )
        .expect("should stage root seed");
    context
}

async fn run_loop(
    root: &str,
    max_cycles: u32,
) -> Result<ConvergeResult, converge_kernel::ConvergeError> {
    let mut engine = build_engine(max_cycles);
    engine.run(seed_root(root)).await
}

fn artifact_profile(target: &str) -> Option<ArtifactProfile> {
    match target {
        "edge-gateway" => Some(ArtifactProfile {
            kind: "service",
            summary: "entrypoint that fans out to auth and catalog reads",
            depends_on: &["auth-service", "catalog-index"],
        }),
        "auth-service" => Some(ArtifactProfile {
            kind: "service",
            summary: "checks credentials and delegates identity lookups",
            depends_on: &["identity-store"],
        }),
        "catalog-index" => Some(ArtifactProfile {
            kind: "service",
            summary: "serves product reads through search and pricing data",
            depends_on: &["search-shard", "pricing-engine"],
        }),
        "identity-store" => Some(ArtifactProfile {
            kind: "datastore",
            summary: "canonical identity records",
            depends_on: &[],
        }),
        "search-shard" => Some(ArtifactProfile {
            kind: "index",
            summary: "full-text shard serving catalog lookups",
            depends_on: &[],
        }),
        "pricing-engine" => Some(ArtifactProfile {
            kind: "service",
            summary: "resolves price rules and currency conversions",
            depends_on: &["currency-feed"],
        }),
        "currency-feed" => Some(ArtifactProfile {
            kind: "feed",
            summary: "daily exchange-rate import",
            depends_on: &[],
        }),
        _ => None,
    }
}

fn pending_surveys(ctx: &dyn Context) -> Vec<SurveyRequest> {
    let mut pending = Vec::new();
    let mut seen = BTreeSet::new();

    for root in roots(ctx) {
        if seen.insert((root.clone(), root.clone())) {
            pending.push(SurveyRequest {
                root: root.clone(),
                target: root,
                reason: "initial root artifact".to_string(),
            });
        }
    }

    for fact in ctx.get(ContextKey::Strategies) {
        if !fact.id.starts_with(REQUEST_PREFIX) {
            continue;
        }

        if let Ok(request) = serde_json::from_str::<SurveyRequest>(&fact.content)
            && seen.insert((request.root.clone(), request.target.clone()))
        {
            pending.push(request);
        }
    }

    pending
}

fn roots(ctx: &dyn Context) -> Vec<String> {
    let mut roots = BTreeSet::new();
    for fact in ctx.get(ContextKey::Seeds) {
        if let Some(root) = fact.id.strip_prefix(ROOT_PREFIX) {
            roots.insert(root.to_string());
        }
    }
    roots.into_iter().collect()
}

fn signals(ctx: &dyn Context) -> Vec<ArtifactSignal> {
    ctx.get(ContextKey::Signals)
        .iter()
        .filter(|fact| fact.id.starts_with(SIGNAL_PREFIX))
        .filter_map(|fact| serde_json::from_str::<ArtifactSignal>(&fact.content).ok())
        .collect()
}

fn request_exists(ctx: &dyn Context, root: &str, target: &str) -> bool {
    let request_id = request_id(root, target);
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|fact| fact.id == request_id)
}

fn signal_exists(ctx: &dyn Context, root: &str, target: &str) -> bool {
    let signal_id = signal_id(root, target);
    ctx.get(ContextKey::Signals)
        .iter()
        .any(|fact| fact.id == signal_id)
}

fn summary_exists(ctx: &dyn Context, root: &str) -> bool {
    let summary_id = summary_id(root);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|fact| fact.id == summary_id)
}

fn closure_inputs(ctx: &dyn Context, root: &str) -> Option<ClosureSummary> {
    if !signal_exists(ctx, root, root) {
        return None;
    }

    let mut surveyed = BTreeSet::new();
    let mut open_dependencies = BTreeSet::new();
    let mut pending_requests = BTreeSet::new();

    for signal in signals(ctx)
        .into_iter()
        .filter(|signal| signal.root == root)
    {
        surveyed.insert(signal.target.clone());
        for dependency in signal.depends_on {
            if !signal_exists(ctx, root, &dependency) {
                open_dependencies.insert(dependency);
            }
        }
    }

    for fact in ctx.get(ContextKey::Strategies) {
        if !fact.id.starts_with(REQUEST_PREFIX) {
            continue;
        }

        let Ok(request) = serde_json::from_str::<SurveyRequest>(&fact.content) else {
            continue;
        };
        if request.root == root && !surveyed.contains(&request.target) {
            pending_requests.insert(request.target);
        }
    }

    if !open_dependencies.is_empty() || !pending_requests.is_empty() {
        return None;
    }

    let rehuddle_count = ctx
        .get(ContextKey::Hypotheses)
        .iter()
        .filter(|fact| fact.id.starts_with(&format!("{GAP_PREFIX}{root}:")))
        .count();

    Some(ClosureSummary {
        root: root.to_string(),
        surveyed: surveyed.into_iter().collect(),
        rehuddle_count,
        pending_requests: pending_requests.into_iter().collect(),
        open_dependencies: open_dependencies.into_iter().collect(),
    })
}

fn parse_summary(ctx: &dyn Context, root: &str) -> Option<ClosureSummary> {
    let summary_id = summary_id(root);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .find(|fact| fact.id == summary_id)
        .and_then(|fact| serde_json::from_str(&fact.content).ok())
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

fn root_id(root: &str) -> String {
    format!("{ROOT_PREFIX}{root}")
}

fn request_id(root: &str, target: &str) -> String {
    format!("{REQUEST_PREFIX}{root}:{target}")
}

fn signal_id(root: &str, target: &str) -> String {
    format!("{SIGNAL_PREFIX}{root}:{target}")
}

fn gap_id(root: &str, source: &str, missing: &str) -> String {
    format!("{GAP_PREFIX}{root}:{source}:{missing}")
}

fn summary_id(root: &str) -> String {
    format!("{SUMMARY_PREFIX}{root}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{ConvergeError, StopReason};
    use std::collections::HashSet;

    #[tokio::test]
    async fn deep_graph_discovers_gaps_until_fixed_point() {
        let result = run_loop("edge-gateway", 20).await.expect("loop should run");
        let summary = parse_summary(&result.context, "edge-gateway").expect("closure summary");

        assert!(result.converged);
        assert!(matches!(result.stop_reason, StopReason::Converged));
        assert!(result.cycles >= 5);
        assert_eq!(
            summary.surveyed,
            vec![
                "auth-service",
                "catalog-index",
                "currency-feed",
                "edge-gateway",
                "identity-store",
                "pricing-engine",
                "search-shard",
            ]
        );
        assert!(summary.rehuddle_count >= 5);
    }

    #[tokio::test]
    async fn graph_shape_changes_cycle_count() {
        let shallow = run_loop("identity-store", 20)
            .await
            .expect("shallow loop should run");
        let deep = run_loop("edge-gateway", 20)
            .await
            .expect("deep loop should run");

        assert!(shallow.converged);
        assert!(deep.converged);
        assert!(deep.cycles > shallow.cycles);
    }

    #[tokio::test]
    async fn low_cycle_budget_stops_before_closure() {
        let error = run_loop("edge-gateway", 3)
            .await
            .expect_err("loop should stop on budget");

        assert!(matches!(
            error,
            ConvergeError::BudgetExhausted { ref kind } if kind == "max_cycles (3)"
        ));
    }

    #[tokio::test]
    async fn rerun_is_idempotent_after_closure() {
        let first = run_loop("edge-gateway", 20)
            .await
            .expect("first run should converge");
        let expected_signal_ids: HashSet<_> = first
            .context
            .get(ContextKey::Signals)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();
        let expected_strategy_ids: HashSet<_> = first
            .context
            .get(ContextKey::Strategies)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();
        let expected_diagnostic_ids: HashSet<_> = first
            .context
            .get(ContextKey::Diagnostic)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();

        let mut rerun_engine = build_engine(20);
        let second = rerun_engine
            .run(first.context.clone())
            .await
            .expect("rerun should converge");

        let rerun_signal_ids: HashSet<_> = second
            .context
            .get(ContextKey::Signals)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();
        let rerun_strategy_ids: HashSet<_> = second
            .context
            .get(ContextKey::Strategies)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();
        let rerun_diagnostic_ids: HashSet<_> = second
            .context
            .get(ContextKey::Diagnostic)
            .iter()
            .map(|fact| fact.id.clone())
            .collect();

        assert_eq!(expected_signal_ids, rerun_signal_ids);
        assert_eq!(expected_strategy_ids, rerun_strategy_ids);
        assert_eq!(expected_diagnostic_ids, rerun_diagnostic_ids);
    }
}
