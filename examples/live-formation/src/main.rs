//! Live Formation: Market Entry Decision
//!
//! A new market opportunity arrives as a signal. Nothing is pre-wired.
//! The engine self-assembles its team — selecting backends for each
//! capability, assigning suggestors to each role — then converges across
//! five specialised agents to a unified go/no-go recommendation.
//!
//! Convergence unfolds in four phases across multiple cycles:
//!
//!   Phase 1 — Self-Assembly
//!     ProviderSelectionSuggestor  maps capability slots → available backends
//!     FormationAssemblySuggestor  maps required roles   → catalog suggestors
//!
//!   Phase 2 — Analysis  (depends on formation plan)
//!     MarketAnalyser     → market sizing and growth outlook
//!     TrendForecaster    → 3-year revenue projection (priority-weighted)
//!     CompetitiveScanner → competitive intensity and risk score
//!
//!   Phase 3 — Gate + Budget
//!     InvestmentGuard    → blocks if competitive risk is too high
//!     BudgetAllocator    → optimal $5M launch spend across 5 channels
//!
//!   Phase 4 — Synthesis  (waits for constraint gate to settle)
//!     LaunchDirector     → go/no-go with rationale, citing formation + providers

use std::sync::Arc;

use async_trait::async_trait;
use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
    formation::{
        CostClass, DeliberatedFormationTemplate, FormationAssemblySuggestor, FormationCatalog,
        FormationPlan, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
        LatencyClass, ProfileSnapshot, ProviderAssignment, ProviderRequest,
        ProviderSelectionSuggestor, SuggestorCapability, SuggestorRole,
    },
};
use converge_provider_api::{Backend, BackendKind, Capability};

const MARKET_ENTRY_TEMPLATE_ID: &str = "market-entry";

// ── Mock backends (stand-ins for live integrations) ───────────────────────────

macro_rules! mock_backend {
    ($name:ident, $label:literal, $kind:expr, $caps:expr) => {
        struct $name;
        impl Backend for $name {
            fn name(&self) -> &str {
                $label
            }
            fn kind(&self) -> BackendKind {
                $kind
            }
            fn capabilities(&self) -> Vec<Capability> {
                $caps
            }
            fn supports_replay(&self) -> bool {
                false
            }
            fn requires_network(&self) -> bool {
                true
            }
        }
    };
}

mock_backend!(
    ClaudeBackend,
    "claude-sonnet-4-6",
    BackendKind::Llm,
    vec![Capability::Reasoning, Capability::CodeGeneration]
);
mock_backend!(
    GeminiBackend,
    "gemini-pro",
    BackendKind::Llm,
    vec![Capability::Reasoning, Capability::MultilingualText]
);
mock_backend!(
    BigQueryBackend,
    "bigquery-analytics",
    BackendKind::Analytics,
    vec![
        Capability::AnomalyDetection,
        Capability::Regression,
        Capability::FullTextSearch
    ]
);
mock_backend!(
    ORToolsBackend,
    "ortools-optimizer",
    BackendKind::Optimization,
    vec![
        Capability::MathematicalProgramming,
        Capability::ResourceAllocation,
        Capability::Scheduling
    ]
);

// ── Catalog helpers ───────────────────────────────────────────────────────────

fn snap(
    name: &str,
    role: SuggestorRole,
    caps: &[SuggestorCapability],
    output_keys: &[ContextKey],
) -> ProfileSnapshot {
    ProfileSnapshot {
        name: name.to_string(),
        role,
        output_keys: output_keys.to_vec(),
        cost_hint: CostClass::Medium,
        latency_hint: LatencyClass::Interactive,
        capabilities: caps.to_vec(),
        confidence_min: 0.5,
        confidence_max: 0.95,
    }
}

// ── Phase 1: Seeder ───────────────────────────────────────────────────────────

struct OpportunitySeeder {
    formation_catalog: FormationCatalog,
}

#[async_trait]
impl Suggestor for OpportunitySeeder {
    fn name(&self) -> &str {
        "opportunity-seeder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        // The raw market opportunity signal.
        let market_signal = serde_json::json!({
            "market": "Enterprise AI Security",
            "region": "EMEA",
            "regulatory_pressure": "high",
            "incumbent_strength": "medium",
            "buyer_readiness": "high",
            "deal_size_avg_usd": 180_000,
            "market_size_usd_bn": 8.4,
            "cagr_pct": 28,
            "win_rate_comparable": 0.34,
            "competitors": ["Wiz", "Orca Security", "Lacework", "Snyk"],
            "launch_budget_usd": 5_000_000
        });

        let query = FormationTemplateQuery::new()
            .with_keyword("market")
            .with_keyword("launch")
            .with_entity("market")
            .with_entity("competitors")
            .with_required_capability(SuggestorCapability::LlmReasoning)
            .with_required_capability(SuggestorCapability::PolicyEnforcement);
        let Some(template) = self.formation_catalog.top_match(&query) else {
            let diagnostic = serde_json::json!({
                "request_id": "launch",
                "message": "no formation template matched the market-entry signal",
            });
            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Diagnostic,
                    "formation-template-miss:launch",
                    diagnostic.to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        };
        let formation_req = template.to_request("launch");
        let template_selection = serde_json::json!({
            "request_id": "launch",
            "template_id": template.id(),
            "template_kind": template.kind(),
            "required_roles": formation_req.required_roles.clone(),
        });

        // Provider request: which capabilities do we need?
        let provider_req = ProviderRequest {
            id: "launch".to_string(),
            required_capabilities: vec![
                Capability::Reasoning,
                Capability::AnomalyDetection,
                Capability::MathematicalProgramming,
            ],
            backend_requirements: None,
        };

        AgentEffect::with_proposals(vec![
            ProposedFact::new(
                ContextKey::Seeds,
                "market-signal",
                market_signal.to_string(),
                self.name(),
            ),
            ProposedFact::new(
                ContextKey::Seeds,
                "formation-request:launch",
                serde_json::to_string(&formation_req).unwrap(),
                self.name(),
            ),
            ProposedFact::new(
                ContextKey::Diagnostic,
                "formation-template:launch",
                template_selection.to_string(),
                self.name(),
            ),
            ProposedFact::new(
                ContextKey::Seeds,
                "provider-request:launch",
                serde_json::to_string(&provider_req).unwrap(),
                self.name(),
            ),
        ])
    }
}

// ── Phase 2: Analysis agents ──────────────────────────────────────────────────
// Each agent checks that the formation plan has been assembled before running.
// This makes formation membership visible in the execution trace.

fn formation_plan(ctx: &dyn Context) -> Option<FormationPlan> {
    ctx.get(ContextKey::Strategies)
        .iter()
        .find(|f| f.id == "formation-plan:launch")
        .and_then(|f| serde_json::from_str(&f.content).ok())
}

fn in_formation(ctx: &dyn Context, suggestor_name: &str) -> bool {
    formation_plan(ctx).is_some_and(|p| p.assignments.iter().any(|a| a.suggestor == suggestor_name))
}

// Market sizing and growth analysis.
struct MarketAnalyser;

#[async_trait]
impl Suggestor for MarketAnalyser {
    fn name(&self) -> &str {
        "market-analyser"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        in_formation(ctx, "market-analyser")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|f| f.id == "analysis:market")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signal: serde_json::Value = ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == "market-signal")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let tam_bn = signal["market_size_usd_bn"].as_f64().unwrap_or(0.0);
        let cagr = signal["cagr_pct"].as_f64().unwrap_or(0.0) / 100.0;

        // SAM = 10% of TAM (realistic penetrable segment for a new entrant).
        // SOM = 5% of SAM at year-3 maturity.
        let sam_m = tam_bn * 1_000.0 * 0.10;
        let som_y3_m = sam_m * 0.05;
        let tam_y3_bn = tam_bn * (1.0 + cagr).powi(3);

        let analysis = serde_json::json!({
            "market": signal["market"],
            "region": signal["region"],
            "tam_current_usd_bn": tam_bn,
            "tam_year3_usd_bn": (tam_y3_bn * 10.0).round() / 10.0,
            "sam_usd_m": sam_m.round(),
            "som_year3_usd_m": (som_y3_m * 10.0).round() / 10.0,
            "cagr_pct": signal["cagr_pct"],
            "buyer_readiness": signal["buyer_readiness"],
            "deal_size_avg_usd": signal["deal_size_avg_usd"],
            "assessment": if tam_bn > 5.0 { "attractive" } else { "marginal" }
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "analysis:market",
                analysis.to_string(),
                self.name(),
            )
            .with_confidence(0.88),
        )
    }
}

// 3-year revenue projection using priority-weighted growth modelling.
struct TrendForecaster;

#[async_trait]
impl Suggestor for TrendForecaster {
    fn name(&self) -> &str {
        "trend-forecaster"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        in_formation(ctx, "trend-forecaster")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|f| f.id == "eval:trend-forecast")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signal: serde_json::Value = ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == "market-signal")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let win_rate = signal["win_rate_comparable"].as_f64().unwrap_or(0.30);
        let deal_size = signal["deal_size_avg_usd"].as_f64().unwrap_or(100_000.0);
        let budget = signal["launch_budget_usd"].as_f64().unwrap_or(5_000_000.0);

        // Pipeline ramp: sales headcount funded by budget, each rep closes ~3 deals/yr.
        let sales_budget = budget * 0.30;
        let sales_reps = (sales_budget / 250_000.0).floor();
        let pipeline_multiplier = 4.0; // 4× pipeline to quota

        let deals_y1 = (sales_reps * 3.0 * win_rate).ceil() as u32;
        let deals_y2 = (deals_y1 as f64 * 2.2).ceil() as u32;
        let deals_y3 = (deals_y2 as f64 * 1.8).ceil() as u32;

        let arr_y1 = deals_y1 as f64 * deal_size;
        let arr_y2 = deals_y2 as f64 * deal_size;
        let arr_y3 = deals_y3 as f64 * deal_size;

        let forecast = serde_json::json!({
            "sales_reps_funded": sales_reps,
            "pipeline_coverage": pipeline_multiplier,
            "arr_year1_usd": arr_y1,
            "arr_year2_usd": arr_y2,
            "arr_year3_usd": arr_y3,
            "break_even_month": if arr_y1 > budget * 0.4 { 18 } else { 30 },
            "confidence": "medium"
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "eval:trend-forecast",
                forecast.to_string(),
                self.name(),
            )
            .with_confidence(0.74),
        )
    }
}

// Competitive intensity and threat scoring.
struct CompetitiveScanner;

#[async_trait]
impl Suggestor for CompetitiveScanner {
    fn name(&self) -> &str {
        "competitive-scanner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        in_formation(ctx, "competitive-scanner")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|f| f.id == "eval:competitive-risk")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signal: serde_json::Value = ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == "market-signal")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let competitors: Vec<&str> = signal["competitors"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let incumbent_strength = signal["incumbent_strength"].as_str().unwrap_or("high");

        // Risk = (competitor count × 0.1) + incumbent factor, clamped to [0, 1].
        let incumbent_factor = match incumbent_strength {
            "low" => 0.10,
            "medium" => 0.25,
            "high" => 0.50,
            _ => 0.30,
        };
        let risk_score = ((competitors.len() as f64 * 0.10) + incumbent_factor).min(1.0);

        let level = if risk_score < 0.40 {
            "low"
        } else if risk_score < 0.65 {
            "medium"
        } else {
            "high"
        };

        let eval = serde_json::json!({
            "competitor_count": competitors.len(),
            "named_competitors": competitors,
            "incumbent_strength": incumbent_strength,
            "risk_score": (risk_score * 100.0).round() / 100.0,
            "risk_level": level,
            "key_threat": if incumbent_strength == "high" {
                "entrenched incumbents with deep EMEA relationships"
            } else {
                "fragmented market with fast-moving challengers"
            }
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "eval:competitive-risk",
                eval.to_string(),
                self.name(),
            )
            .with_confidence(0.82),
        )
    }
}

// ── Phase 3a: Investment gate ─────────────────────────────────────────────────

struct InvestmentGuard;

#[async_trait]
impl Suggestor for InvestmentGuard {
    fn name(&self) -> &str {
        "investment-guard"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let evals = ctx.get(ContextKey::Evaluations);
        let has_all = [
            "analysis:market",
            "eval:trend-forecast",
            "eval:competitive-risk",
        ]
        .iter()
        .all(|id| evals.iter().any(|f| f.id == *id));
        in_formation(ctx, "investment-guard") && has_all && !ctx.has(ContextKey::Constraints)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let evals = ctx.get(ContextKey::Evaluations);

        let risk_score: f64 = evals
            .iter()
            .find(|f| f.id == "eval:competitive-risk")
            .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
            .and_then(|v| v["risk_score"].as_f64())
            .unwrap_or(1.0);

        let tam_bn: f64 = evals
            .iter()
            .find(|f| f.id == "analysis:market")
            .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
            .and_then(|v| v["tam_current_usd_bn"].as_f64())
            .unwrap_or(0.0);

        let blocked = risk_score > 0.65 || tam_bn < 1.0;

        let gate = serde_json::json!({
            "gate": "investment-risk",
            "decision": if blocked { "block" } else { "permit" },
            "risk_score": risk_score,
            "tam_usd_bn": tam_bn,
            "reasons": if blocked {
                vec![
                    if risk_score > 0.65 { Some("competitive risk exceeds threshold (>0.65)") } else { None },
                    if tam_bn < 1.0 { Some("TAM too small (<$1B)") } else { None },
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
            } else {
                vec!["competitive risk within tolerance", "TAM sufficient"]
            }
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Constraints,
                "risk-gate",
                gate.to_string(),
                self.name(),
            )
            .with_confidence(0.95),
        )
    }
}

// ── Phase 3b: Budget allocation ───────────────────────────────────────────────

struct BudgetAllocator;

#[async_trait]
impl Suggestor for BudgetAllocator {
    fn name(&self) -> &str {
        "budget-allocator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "analysis:market")
            && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signal: serde_json::Value = ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == "market-signal")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let budget = signal["launch_budget_usd"].as_f64().unwrap_or(5_000_000.0);

        // Priority weights drive allocation. Higher weight = more budget.
        // Weights informed by win-rate sensitivity analysis for enterprise SaaS.
        let channels: &[(&str, &str, f64)] = &[
            ("product", "Product & Engineering", 0.38),
            ("sales", "Sales & Partnerships", 0.28),
            ("marketing", "Marketing & Brand", 0.16),
            ("cx", "Customer Experience", 0.11),
            ("compliance", "Legal & Compliance", 0.07),
        ];

        let allocations: Vec<serde_json::Value> = channels
            .iter()
            .map(|(id, name, weight)| {
                let amount = (budget * weight).round();
                serde_json::json!({
                    "channel": id,
                    "name": name,
                    "amount_usd": amount,
                    "pct": (*weight * 100.0).round(),
                    "rationale": format!("{:.0}% priority weight (enterprise SaaS baseline)", weight * 100.0)
                })
            })
            .collect();

        let plan = serde_json::json!({
            "total_budget_usd": budget,
            "method": "priority-weighted allocation",
            "channels": allocations,
            "optimization_note": "weights calibrated against EMEA enterprise SaaS benchmarks"
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Hypotheses,
                "plan:budget",
                plan.to_string(),
                self.name(),
            )
            .with_confidence(0.80),
        )
    }
}

// ── Phase 4: Synthesis ────────────────────────────────────────────────────────

struct LaunchDirector;

#[async_trait]
impl Suggestor for LaunchDirector {
    fn name(&self) -> &str {
        "launch-director"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        in_formation(ctx, "launch-director")
            && ctx.has(ContextKey::Constraints)
            && ctx.has(ContextKey::Hypotheses)
            && !ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        // Collect all upstream artefacts.
        let gate: serde_json::Value = ctx
            .get(ContextKey::Constraints)
            .iter()
            .find(|f| f.id == "risk-gate")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let market: serde_json::Value = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "analysis:market")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let forecast: serde_json::Value = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "eval:trend-forecast")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let risk: serde_json::Value = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "eval:competitive-risk")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let budget: serde_json::Value = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .find(|f| f.id == "plan:budget")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        // Formation provenance from the plan written by FormationAssemblySuggestor.
        let formation: Vec<String> = formation_plan(ctx)
            .map(|p| {
                p.assignments
                    .iter()
                    .map(|a| format!("{}→{:?}", a.suggestor, a.role))
                    .collect()
            })
            .unwrap_or_default();

        // Provider provenance from the assignment written by ProviderSelectionSuggestor.
        let provider_assignment: Option<ProviderAssignment> = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|f| f.id == "provider-assignment:launch")
            .and_then(|f| serde_json::from_str(&f.content).ok());

        let providers: Vec<String> = provider_assignment
            .map(|a| {
                a.assignments
                    .iter()
                    .map(|p| format!("{}→{:?}", p.backend_name, p.capability))
                    .collect()
            })
            .unwrap_or_default();

        let gated = gate["decision"].as_str().unwrap_or("block") == "block";
        let verdict = if gated { "NO-GO" } else { "GO" };
        let confidence: f64 = if gated { 0.91 } else { 0.83 };

        let recommendation = serde_json::json!({
            "verdict": verdict,
            "confidence": confidence,
            "market": {
                "tam_usd_bn": market["tam_current_usd_bn"],
                "som_year3_usd_m": market["som_year3_usd_m"],
                "assessment": market["assessment"]
            },
            "revenue_outlook": {
                "arr_year1": forecast["arr_year1_usd"],
                "arr_year3": forecast["arr_year3_usd"],
                "break_even_month": forecast["break_even_month"]
            },
            "risk": {
                "level": risk["risk_level"],
                "score": risk["risk_score"],
                "key_threat": risk["key_threat"]
            },
            "budget_plan": budget["channels"],
            "gate_decision": gate["decision"],
            "formation_assembled": formation,
            "providers_assigned": providers
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Proposals,
                "recommendation:launch",
                recommendation.to_string(),
                self.name(),
            )
            .with_confidence(confidence),
        )
    }
}

// ── Engine assembly and run ───────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║          Live Formation: Market Entry Decision               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ── Backend pool ──────────────────────────────────────────────────────────
    // In production: Arc<AnthropicBackend>, Arc<GeminiBackend>, etc.
    let backends: Vec<Arc<dyn Backend>> = vec![
        Arc::new(ClaudeBackend),
        Arc::new(GeminiBackend),
        Arc::new(BigQueryBackend),
        Arc::new(ORToolsBackend),
    ];

    println!("Backend pool ({} registered):", backends.len());
    for b in &backends {
        println!(
            "  {:<28} {:?}",
            b.name(),
            b.capabilities()
                .iter()
                .map(|c| format!("{c:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();

    // ── Formation template catalog ───────────────────────────────────────────
    let formation_catalog = FormationCatalog::new().with_template(FormationTemplate::deliberated(
        DeliberatedFormationTemplate::new(
            FormationTemplateMetadata::new(
                MARKET_ENTRY_TEMPLATE_ID,
                "Market entry go/no-go with policy and synthesis coverage",
                [
                    SuggestorRole::Analysis,
                    SuggestorRole::Planning,
                    SuggestorRole::Evaluation,
                    SuggestorRole::Constraint,
                    SuggestorRole::Synthesis,
                ],
            )
            .with_keyword("market")
            .with_keyword("launch")
            .with_keyword("entry")
            .with_entity("market")
            .with_entity("region")
            .with_entity("competitors")
            .with_required_capability(SuggestorCapability::LlmReasoning)
            .with_required_capability(SuggestorCapability::PolicyEnforcement),
            3,
        ),
    ));

    println!(
        "Formation template catalog ({} registered):",
        formation_catalog.len()
    );
    for template in formation_catalog.iter() {
        println!(
            "  {:<24} kind={:?}  roles={}",
            template.id(),
            template.kind(),
            template.metadata().required_roles.len()
        );
    }
    println!();

    // ── Suggestor catalog (for formation assembly) ────────────────────────────
    let catalog = vec![
        snap(
            "market-analyser",
            SuggestorRole::Analysis,
            &[SuggestorCapability::LlmReasoning],
            &[ContextKey::Evaluations],
        ),
        snap(
            "trend-forecaster",
            SuggestorRole::Planning,
            &[
                SuggestorCapability::Analytics,
                SuggestorCapability::Optimization,
            ],
            &[ContextKey::Evaluations],
        ),
        snap(
            "competitive-scanner",
            SuggestorRole::Evaluation,
            &[
                SuggestorCapability::LlmReasoning,
                SuggestorCapability::Analytics,
            ],
            &[ContextKey::Evaluations],
        ),
        snap(
            "investment-guard",
            SuggestorRole::Constraint,
            &[SuggestorCapability::PolicyEnforcement],
            &[ContextKey::Constraints],
        ),
        snap(
            "launch-director",
            SuggestorRole::Synthesis,
            &[SuggestorCapability::LlmReasoning],
            &[ContextKey::Proposals],
        ),
    ];

    println!("Suggestor catalog ({} registered):", catalog.len());
    for s in &catalog {
        println!(
            "  {:<26} role={:?}  caps=[{}]",
            s.name,
            s.role,
            s.capabilities
                .iter()
                .map(|c| format!("{c:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();

    // ── Engine ────────────────────────────────────────────────────────────────
    let mut engine = Engine::with_budget(Budget {
        max_cycles: 12,
        max_facts: 500,
    });

    // Phase 1: seeder
    engine.register_suggestor(OpportunitySeeder { formation_catalog });

    // Phase 1: self-assembly (both read Seeds → write Strategies)
    engine.register_suggestor(ProviderSelectionSuggestor::new(backends));
    engine.register_suggestor(FormationAssemblySuggestor::new(catalog));

    // Phase 2: analysis (depend on Strategies — wait for formation plan)
    engine.register_suggestor(MarketAnalyser);
    engine.register_suggestor(TrendForecaster);
    engine.register_suggestor(CompetitiveScanner);

    // Phase 3: gate + budget (depend on Evaluations)
    engine.register_suggestor(InvestmentGuard);
    engine.register_suggestor(BudgetAllocator);

    // Phase 4: synthesis (depends on Constraints)
    engine.register_suggestor(LaunchDirector);

    println!("Running convergence...");
    println!("─────────────────────────────────────────────────────────────");

    let result = engine
        .run(ContextState::new())
        .await
        .expect("engine should converge");

    println!(
        "Converged: {}  cycles: {}  stop: {:?}",
        result.converged, result.cycles, result.stop_reason
    );
    println!();

    // ── Self-assembly output ──────────────────────────────────────────────────

    println!("╔══ Phase 1: Self-Assembly ══════════════════════════════════════╗");
    println!();

    if let Some(assignment) = result
        .context
        .get(ContextKey::Strategies)
        .iter()
        .find(|f| f.id == "provider-assignment:launch")
        .and_then(|f| serde_json::from_str::<ProviderAssignment>(&f.content).ok())
    {
        println!(
            "  Provider assignment  (coverage: {:.0}%)",
            assignment.coverage_ratio * 100.0
        );
        for a in &assignment.assignments {
            println!("    {:?}  →  {}", a.capability, a.backend_name);
        }
        if !assignment.unmatched.is_empty() {
            println!("    UNMATCHED: {:?}", assignment.unmatched);
        }
    }

    println!();

    if let Some(plan) = result
        .context
        .get(ContextKey::Strategies)
        .iter()
        .find(|f| f.id == "formation-plan:launch")
        .and_then(|f| serde_json::from_str::<FormationPlan>(&f.content).ok())
    {
        println!(
            "  Formation plan  (coverage: {:.0}%)",
            plan.coverage_ratio * 100.0
        );
        for a in &plan.assignments {
            println!("    {:?}  →  {}", a.role, a.suggestor);
        }
        if !plan.unmatched_roles.is_empty() {
            println!("    UNMATCHED ROLES: {:?}", plan.unmatched_roles);
        }
    }

    println!();

    // ── Analysis output ───────────────────────────────────────────────────────

    println!("╔══ Phase 2: Analysis ═══════════════════════════════════════════╗");
    println!();

    for eval in result.context.get(ContextKey::Evaluations) {
        match eval.id.as_str() {
            "analysis:market" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                    println!(
                        "  Market Analysis  [{}]",
                        v["assessment"].as_str().unwrap_or("?")
                    );
                    println!(
                        "    TAM: ${:.1}B now  →  ${:.1}B (year 3)    CAGR: {}%",
                        v["tam_current_usd_bn"].as_f64().unwrap_or(0.0),
                        v["tam_year3_usd_bn"].as_f64().unwrap_or(0.0),
                        v["cagr_pct"],
                    );
                    println!(
                        "    SAM: ${:.0}M       SOM (yr 3): ${:.1}M",
                        v["sam_usd_m"].as_f64().unwrap_or(0.0),
                        v["som_year3_usd_m"].as_f64().unwrap_or(0.0),
                    );
                    println!("    Deal size: ${}", v["deal_size_avg_usd"]);
                }
            }
            "eval:competitive-risk" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                    println!(
                        "  Competitive Risk  [{}]  score: {}",
                        v["risk_level"].as_str().unwrap_or("?"),
                        v["risk_score"].as_f64().unwrap_or(0.0),
                    );
                    println!(
                        "    {} competitors: {}",
                        v["competitor_count"], v["named_competitors"]
                    );
                    println!(
                        "    Key threat: {}",
                        v["key_threat"].as_str().unwrap_or("unknown")
                    );
                }
            }
            "eval:trend-forecast" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                    let arr_y1 = v["arr_year1_usd"].as_f64().unwrap_or(0.0) / 1_000_000.0;
                    let arr_y3 = v["arr_year3_usd"].as_f64().unwrap_or(0.0) / 1_000_000.0;
                    println!("  Revenue Forecast  [confidence: {}]", v["confidence"]);
                    println!(
                        "    Year 1: ${:.1}M ARR   Year 3: ${:.1}M ARR",
                        arr_y1, arr_y3
                    );
                    println!("    Break-even: month {}", v["break_even_month"]);
                    println!("    Sales reps funded: {}", v["sales_reps_funded"]);
                }
            }
            _ => {}
        }
        println!();
    }

    // ── Gate + Budget ─────────────────────────────────────────────────────────

    println!("╔══ Phase 3: Gate + Budget ══════════════════════════════════════╗");
    println!();

    if let Some(gate) = result
        .context
        .get(ContextKey::Constraints)
        .iter()
        .find(|f| f.id == "risk-gate")
        .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
    {
        let decision = gate["decision"].as_str().unwrap_or("block");
        let icon = if decision == "permit" { "✓" } else { "✗" };
        println!("  Investment Gate  {}  [{decision}]", icon);
        if let Some(reasons) = gate["reasons"].as_array() {
            for r in reasons {
                println!("    → {}", r.as_str().unwrap_or("?"));
            }
        }
    }

    println!();

    if let Some(budget) = result
        .context
        .get(ContextKey::Hypotheses)
        .iter()
        .find(|f| f.id == "plan:budget")
        .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
    {
        println!(
            "  Budget Plan  ${:.1}M  [{}]",
            budget["total_budget_usd"].as_f64().unwrap_or(0.0) / 1_000_000.0,
            budget["method"].as_str().unwrap_or("?"),
        );
        if let Some(channels) = budget["channels"].as_array() {
            for c in channels {
                println!(
                    "    {:<30} ${:>8.0}   ({:.0}%)",
                    c["name"].as_str().unwrap_or("?"),
                    c["amount_usd"].as_f64().unwrap_or(0.0),
                    c["pct"].as_f64().unwrap_or(0.0),
                );
            }
        }
    }

    println!();

    // ── Recommendation ────────────────────────────────────────────────────────

    println!("╔══ Phase 4: Recommendation ═════════════════════════════════════╗");
    println!();

    if let Some(rec) = result
        .context
        .get(ContextKey::Proposals)
        .iter()
        .find(|f| f.id == "recommendation:launch")
        .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
    {
        let verdict = rec["verdict"].as_str().unwrap_or("?");
        let confidence = rec["confidence"].as_f64().unwrap_or(0.0);
        let icon = if verdict == "GO" { "▶" } else { "■" };

        println!(
            "  {icon} VERDICT: {verdict}   confidence: {:.0}%",
            confidence * 100.0
        );
        println!();

        println!("  Formation assembled by converge:");
        if let Some(arr) = rec["formation_assembled"].as_array() {
            for s in arr {
                println!("    {}", s.as_str().unwrap_or("?"));
            }
        }

        println!();
        println!("  Providers assigned by converge:");
        if let Some(arr) = rec["providers_assigned"].as_array() {
            for p in arr {
                println!("    {}", p.as_str().unwrap_or("?"));
            }
        }
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
}
