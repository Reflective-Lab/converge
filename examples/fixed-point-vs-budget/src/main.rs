//! Fixed Point vs Budget — same loop, two different stop outcomes.
//!
//! This example demonstrates the actual public kernel contract:
//!
//! - With enough budget, `Engine::run()` returns `ConvergeResult` with
//!   `StopReason::Converged`.
//! - With too little budget, `Engine::run()` returns
//!   `ConvergeError::BudgetExhausted`.
//!
//! It also puts one audited optimization algorithm to real use: the frontier
//! planner uses Dijkstra from `converge-optimization` to choose the nearest
//! unexplored artifact in the discovered graph.

use std::collections::{BTreeSet, HashMap};

use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, ConvergeError, ConvergeResult, Engine,
    Fact, Suggestor,
};
use converge_optimization::graph::{Graph, NodeId, dijkstra::shortest_path};
use serde::{Deserialize, Serialize};

const ROOT_PREFIX: &str = "graph-root:";
const REQUEST_PREFIX: &str = "frontier-request:";
const SIGNAL_PREFIX: &str = "graph-signal:";
const HYPOTHESIS_PREFIX: &str = "frontier-choice:";
const SUMMARY_PREFIX: &str = "fixed-point-summary:";

const SURVEY_DEPENDENCIES: [ContextKey; 2] = [ContextKey::Seeds, ContextKey::Strategies];
const FRONTIER_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Signals];
const SUMMARY_DEPENDENCIES: [ContextKey; 3] = [
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
struct FrontierChoice {
    root: String,
    target: String,
    distance: i64,
    path: Vec<String>,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FixedPointSummary {
    root: String,
    surveyed: Vec<String>,
    request_order: Vec<String>,
    longest_route: Vec<String>,
    longest_distance: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontierCandidate {
    target: String,
    distance: i64,
    path: Vec<String>,
}

struct ArtifactSurveySuggestor;

#[async_trait::async_trait]
impl Suggestor for ArtifactSurveySuggestor {
    fn name(&self) -> &str {
        "artifact-survey"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &SURVEY_DEPENDENCIES
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

            let Some(profile) = artifact_profile(&request.target) else {
                continue;
            };

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
                .with_confidence(0.93),
            );
        }

        AgentEffect::with_proposals(proposals)
    }
}

struct FrontierPlannerSuggestor;

#[async_trait::async_trait]
impl Suggestor for FrontierPlannerSuggestor {
    fn name(&self) -> &str {
        "frontier-planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &FRONTIER_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        roots(ctx)
            .into_iter()
            .any(|root| next_candidate(ctx, &root).is_some())
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for root in roots(ctx) {
            let Some(candidate) = next_candidate(ctx, &root) else {
                continue;
            };

            let choice = FrontierChoice {
                root: root.clone(),
                target: candidate.target.clone(),
                distance: candidate.distance,
                path: candidate.path.clone(),
                reason: format!(
                    "nearest unexplored artifact via {}",
                    candidate.path.join(" -> ")
                ),
            };
            let request = SurveyRequest {
                root: root.clone(),
                target: candidate.target.clone(),
                reason: format!(
                    "chosen by Dijkstra frontier planning; distance={}, route={}",
                    candidate.distance,
                    candidate.path.join(" -> ")
                ),
            };

            proposals.push(
                converge_kernel::ProposedFact::new(
                    ContextKey::Hypotheses,
                    choice_id(&root, &candidate.target),
                    serde_json::to_string(&choice).unwrap_or_default(),
                    self.name(),
                )
                .with_confidence(0.82),
            );
            proposals.push(
                converge_kernel::ProposedFact::new(
                    ContextKey::Strategies,
                    request_id(&root, &candidate.target),
                    serde_json::to_string(&request).unwrap_or_default(),
                    self.name(),
                )
                .with_confidence(0.88),
            );
        }

        AgentEffect::with_proposals(proposals)
    }
}

struct SummarySuggestor;

#[async_trait::async_trait]
impl Suggestor for SummarySuggestor {
    fn name(&self) -> &str {
        "fixed-point-summary"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &SUMMARY_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        roots(ctx)
            .into_iter()
            .any(|root| !summary_exists(ctx, &root) && fixed_point_summary(ctx, &root).is_some())
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for root in roots(ctx) {
            if summary_exists(ctx, &root) {
                continue;
            }

            let Some(summary) = fixed_point_summary(ctx, &root) else {
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
    println!("=== Fixed Point vs Budget ===\n");
    println!("Root: edge-gateway");
    println!("Frontier planning uses real Dijkstra shortest-path routing.\n");

    let converged = run_loop("edge-gateway", 20)
        .await
        .expect("generous budget should converge");
    let summary = parse_summary(&converged.context, "edge-gateway").expect("summary");

    println!("Generous budget:");
    println!(
        "  stop = {:?}, cycles = {}",
        converged.stop_reason, converged.cycles
    );
    println!("  surveyed = {}", summary.surveyed.join(", "));
    println!("  request order = {}", summary.request_order.join(" -> "));
    println!(
        "  longest route = {} (distance {})\n",
        summary.longest_route.join(" -> "),
        summary.longest_distance
    );

    println!("Budget-limited run:");
    match run_loop("edge-gateway", 8).await {
        Ok(result) => {
            println!(
                "  unexpected success: {:?} in {} cycles",
                result.stop_reason, result.cycles
            );
        }
        Err(ConvergeError::BudgetExhausted { kind }) => {
            println!("  stop = BudgetExhausted({kind})");
            println!("  note = partial context is not exposed by Engine::run()\n");
        }
        Err(error) => {
            println!("  unexpected error = {error}");
        }
    }

    print_section("Signals", converged.context.get(ContextKey::Signals));
    print_section("Hypotheses", converged.context.get(ContextKey::Hypotheses));
    print_section("Strategies", converged.context.get(ContextKey::Strategies));
    print_section("Diagnostic", converged.context.get(ContextKey::Diagnostic));
}

fn build_engine(max_cycles: u32) -> Engine {
    let mut engine = Engine::with_budget(Budget {
        max_cycles,
        max_facts: 256,
    });
    engine.register_suggestor(ArtifactSurveySuggestor);
    engine.register_suggestor(FrontierPlannerSuggestor);
    engine.register_suggestor(SummarySuggestor);
    engine
}

fn seed_root(root: &str) -> ContextState {
    let mut context = ContextState::new();
    context
        .add_input_with_provenance(
            ContextKey::Seeds,
            root_id(root),
            root,
            "example:fixed-point-vs-budget",
        )
        .expect("should stage root seed");
    context
}

async fn run_loop(root: &str, max_cycles: u32) -> Result<ConvergeResult, ConvergeError> {
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

fn signals_for_root(ctx: &dyn Context, root: &str) -> Vec<ArtifactSignal> {
    ctx.get(ContextKey::Signals)
        .iter()
        .filter(|fact| fact.id.starts_with(&format!("{SIGNAL_PREFIX}{root}:")))
        .filter_map(|fact| serde_json::from_str::<ArtifactSignal>(&fact.content).ok())
        .collect()
}

fn signal_exists(ctx: &dyn Context, root: &str, target: &str) -> bool {
    ctx.get(ContextKey::Signals)
        .iter()
        .any(|fact| fact.id == signal_id(root, target))
}

fn request_exists(ctx: &dyn Context, root: &str, target: &str) -> bool {
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|fact| fact.id == request_id(root, target))
}

fn summary_exists(ctx: &dyn Context, root: &str) -> bool {
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|fact| fact.id == summary_id(root))
}

fn next_candidate(ctx: &dyn Context, root: &str) -> Option<FrontierCandidate> {
    let (graph, node_ids) = build_discovered_graph(ctx, root)?;
    let root_node = *node_ids.get(root)?;

    let mut candidates: Vec<FrontierCandidate> = Vec::new();
    let mut seen_targets = BTreeSet::new();

    for signal in signals_for_root(ctx, root) {
        for dependency in &signal.depends_on {
            if signal_exists(ctx, root, dependency)
                || request_exists(ctx, root, dependency)
                || !seen_targets.insert(dependency.clone())
            {
                continue;
            }

            let Some(&target_node) = node_ids.get(dependency) else {
                continue;
            };
            let Ok(Some(path)) = shortest_path(&graph, root_node, target_node, |&weight| weight)
            else {
                continue;
            };

            candidates.push(FrontierCandidate {
                target: dependency.clone(),
                distance: path.cost,
                path: path
                    .nodes
                    .into_iter()
                    .map(|node| graph[node].clone())
                    .collect(),
            });
        }
    }

    candidates.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| left.target.cmp(&right.target))
    });
    candidates.into_iter().next()
}

fn build_discovered_graph(
    ctx: &dyn Context,
    root: &str,
) -> Option<(Graph<String, i64>, HashMap<String, NodeId>)> {
    let mut graph: Graph<String, i64> = Graph::new();
    let mut nodes: HashMap<String, NodeId> = HashMap::new();

    ensure_node(&mut graph, &mut nodes, root);
    if !signal_exists(ctx, root, root) {
        return None;
    }

    for signal in signals_for_root(ctx, root) {
        let from = ensure_node(&mut graph, &mut nodes, &signal.target);
        for dependency in signal.depends_on {
            let to = ensure_node(&mut graph, &mut nodes, &dependency);
            if graph.find_edge(from, to).is_none() {
                graph.add_edge(from, to, 1);
            }
        }
    }

    Some((graph, nodes))
}

fn ensure_node(
    graph: &mut Graph<String, i64>,
    nodes: &mut HashMap<String, NodeId>,
    label: &str,
) -> NodeId {
    if let Some(&node) = nodes.get(label) {
        return node;
    }

    let node = graph.add_node(label.to_string());
    nodes.insert(label.to_string(), node);
    node
}

fn fixed_point_summary(ctx: &dyn Context, root: &str) -> Option<FixedPointSummary> {
    if !signal_exists(ctx, root, root) || next_candidate(ctx, root).is_some() {
        return None;
    }

    let pending_requests: Vec<_> = request_order(ctx, root)
        .into_iter()
        .filter(|target| !signal_exists(ctx, root, target))
        .collect();
    if !pending_requests.is_empty() {
        return None;
    }

    let (graph, node_ids) = build_discovered_graph(ctx, root)?;
    let root_node = *node_ids.get(root)?;

    let mut surveyed: Vec<String> = signals_for_root(ctx, root)
        .into_iter()
        .map(|signal| signal.target)
        .collect();
    surveyed.sort();
    surveyed.dedup();

    let mut longest_route = vec![root.to_string()];
    let mut longest_distance = 0;
    for target in &surveyed {
        let Some(&target_node) = node_ids.get(target) else {
            continue;
        };
        let Ok(Some(path)) = shortest_path(&graph, root_node, target_node, |&weight| weight) else {
            continue;
        };
        if path.cost > longest_distance {
            longest_distance = path.cost;
            longest_route = path
                .nodes
                .into_iter()
                .map(|node| graph[node].clone())
                .collect();
        }
    }

    Some(FixedPointSummary {
        root: root.to_string(),
        surveyed,
        request_order: request_order(ctx, root),
        longest_route,
        longest_distance,
    })
}

fn request_order(ctx: &dyn Context, root: &str) -> Vec<String> {
    ctx.get(ContextKey::Strategies)
        .iter()
        .filter(|fact| fact.id.starts_with(&format!("{REQUEST_PREFIX}{root}:")))
        .filter_map(|fact| serde_json::from_str::<SurveyRequest>(&fact.content).ok())
        .map(|request| request.target)
        .collect()
}

fn parse_summary(ctx: &dyn Context, root: &str) -> Option<FixedPointSummary> {
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .find(|fact| fact.id == summary_id(root))
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

fn choice_id(root: &str, target: &str) -> String {
    format!("{HYPOTHESIS_PREFIX}{root}:{target}")
}

fn summary_id(root: &str) -> String {
    format!("{SUMMARY_PREFIX}{root}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::StopReason;

    #[tokio::test]
    async fn generous_budget_reaches_fixed_point() {
        let result = run_loop("edge-gateway", 20).await.expect("should converge");
        let summary = parse_summary(&result.context, "edge-gateway").expect("summary");

        assert!(result.converged);
        assert!(matches!(result.stop_reason, StopReason::Converged));
        assert!(result.cycles >= 12);
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
        assert_eq!(
            summary.longest_route,
            vec![
                "edge-gateway".to_string(),
                "catalog-index".to_string(),
                "pricing-engine".to_string(),
                "currency-feed".to_string(),
            ]
        );
        assert_eq!(summary.longest_distance, 3);
    }

    #[tokio::test]
    async fn frontier_order_uses_shortest_path_then_name_tiebreak() {
        let result = run_loop("edge-gateway", 20).await.expect("should converge");
        let summary = parse_summary(&result.context, "edge-gateway").expect("summary");

        assert_eq!(
            summary.request_order,
            vec![
                "auth-service",
                "catalog-index",
                "identity-store",
                "pricing-engine",
                "search-shard",
                "currency-feed",
            ]
        );
    }

    #[tokio::test]
    async fn low_budget_returns_budget_exhausted() {
        let error = run_loop("edge-gateway", 8)
            .await
            .expect_err("budget should exhaust");

        assert!(matches!(
            error,
            ConvergeError::BudgetExhausted { ref kind } if kind == "max_cycles (8)"
        ));
    }

    #[tokio::test]
    async fn shallow_root_converges_in_fewer_cycles() {
        let shallow = run_loop("identity-store", 20)
            .await
            .expect("shallow root should converge");
        let deep = run_loop("edge-gateway", 20)
            .await
            .expect("deep root should converge");
        let summary = parse_summary(&shallow.context, "identity-store").expect("summary");

        assert!(shallow.cycles < deep.cycles);
        assert_eq!(summary.request_order, Vec::<String>::new());
        assert_eq!(summary.longest_distance, 0);
    }
}
