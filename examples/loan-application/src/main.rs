// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Loan Application — parallel verification with conditional HITL.
//!
//! Demonstrates: parallel execution, conditional HITL (borderline cases), consensus.

use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, EngineHitlPolicy, GateDecision,
    ProposedFact, RunResult, Suggestor, TimeoutAction, TimeoutPolicy,
};

struct ApplicationIngestionAgent;

#[async_trait::async_trait]
impl Suggestor for ApplicationIngestionAgent {
    fn name(&self) -> &str {
        "ApplicationIngestionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let seed = seeds.first();

        if let Some(s) = seed
            && let Ok(app) = serde_json::from_str::<serde_json::Value>(&s.content)
        {
            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Signals,
                    "application",
                    app.to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::empty()
    }
}

struct DocumentVerificationAgent;

#[async_trait::async_trait]
impl Suggestor for DocumentVerificationAgent {
    fn name(&self) -> &str {
        "DocumentVerificationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let signal = signals.first();

        if let Some(s) = signal
            && let Ok(app) = serde_json::from_str::<serde_json::Value>(&s.content)
        {
            let docs_complete = app
                .get("documents")
                .and_then(|d| d.as_bool())
                .unwrap_or(false);

            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Evaluations,
                    "documents",
                    serde_json::json!({
                        "criterion": "documents",
                        "score": if docs_complete { 1.0 } else { 0.0 },
                        "details": if docs_complete { "All required documents provided" } else { "Missing documents" }
                    }).to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::empty()
    }
}

struct CreditCheckAgent;

#[async_trait::async_trait]
impl Suggestor for CreditCheckAgent {
    fn name(&self) -> &str {
        "CreditCheckAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let signal = signals.first();

        if let Some(s) = signal
            && let Ok(app) = serde_json::from_str::<serde_json::Value>(&s.content)
        {
            let credit_score: u32 = app
                .get("credit_score")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let income: f64 = app.get("income").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let requested: f64 = app
                .get("requested_amount")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let credit_score_score = if credit_score >= 750 {
                1.0
            } else if credit_score >= 700 {
                0.8
            } else if credit_score >= 650 {
                0.6
            } else if credit_score >= 600 {
                0.4
            } else {
                0.1
            };

            let dti = requested / income;
            let dti_score = if dti < 0.2 {
                1.0
            } else if dti < 0.3 {
                0.8
            } else if dti < 0.4 {
                0.5
            } else {
                0.2
            };

            let combined = (credit_score_score + dti_score) / 2.0;

            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Evaluations,
                    "credit",
                    serde_json::json!({
                        "criterion": "credit",
                        "score": combined,
                        "details": {
                            "credit_score": credit_score,
                            "dti_ratio": dti
                        }
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::empty()
    }
}

struct ComplianceAgent;

#[async_trait::async_trait]
impl Suggestor for ComplianceAgent {
    fn name(&self) -> &str {
        "ComplianceAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let signal = signals.first();

        if let Some(s) = signal
            && let Ok(app) = serde_json::from_str::<serde_json::Value>(&s.content)
        {
            let us_citizen = app
                .get("us_citizen")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let age = app.get("age").and_then(|v| v.as_u64()).unwrap_or(0);
            let bankruptcies = app
                .get("bankruptcies")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let mut violations = Vec::new();
            if !us_citizen {
                violations.push("Not a US citizen");
            }
            if age < 18 {
                violations.push("Under 18 years old");
            }
            if bankruptcies > 0 {
                violations.push("Recent bankruptcies");
            }

            let compliant = violations.is_empty();

            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Evaluations,
                    "compliance",
                    serde_json::json!({
                        "criterion": "compliance",
                        "score": if compliant { 1.0 } else { 0.0 },
                        "details": {
                            "violations": violations
                        }
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::empty()
    }
}

struct RiskAssessmentAgent;

#[async_trait::async_trait]
impl Suggestor for RiskAssessmentAgent {
    fn name(&self) -> &str {
        "RiskAssessmentAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let signal = signals.first();

        if let Some(s) = signal
            && let Ok(app) = serde_json::from_str::<serde_json::Value>(&s.content)
        {
            let employment_years: u32 = app
                .get("employment_years")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let requested: f64 = app
                .get("requested_amount")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let risk_factors = if employment_years < 1 {
                vec!["Less than 1 year employment"]
            } else if requested > 500000.0 {
                vec!["High loan amount"]
            } else {
                vec![]
            };

            let risk_score = if risk_factors.is_empty() {
                1.0
            } else if risk_factors.len() == 1 {
                0.6
            } else {
                0.3
            };

            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Evaluations,
                    "risk",
                    serde_json::json!({
                        "criterion": "risk",
                        "score": risk_score,
                        "details": {
                            "risk_factors": risk_factors
                        }
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::empty()
    }
}

struct LoanDecisionAgent;

#[async_trait::async_trait]
impl Suggestor for LoanDecisionAgent {
    fn name(&self) -> &str {
        "LoanDecisionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Evaluations) && !ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);

        let mut total_score = 0.0;
        let mut count = 0;

        for eval in evaluations {
            if let Ok(e) = serde_json::from_str::<serde_json::Value>(&eval.content)
                && let Some(score) = e.get("score").and_then(|v| v.as_f64())
            {
                total_score += score;
                count += 1;
            }
        }

        let avg_score = if count > 0 {
            total_score / count as f64
        } else {
            0.0
        };

        let (decision, confidence) = if avg_score >= 0.8 {
            ("approved", 0.95)
        } else if avg_score >= 0.6 {
            ("borderline", 0.70)
        } else {
            ("rejected", 0.90)
        };

        let proposal = ProposedFact {
            key: ContextKey::Proposals,
            id: "loan-decision".into(),
            content: serde_json::json!({
                "decision": decision,
                "score": avg_score,
                "confidence": confidence
            })
            .to_string(),
            confidence,
            provenance: "loan-decision-agent".to_string(),
        };

        AgentEffect::with_proposal(proposal)
    }
}

#[tokio::main]
async fn main() {
    println!("=== Loan Application Example ===\n");

    let mut engine = Engine::new();

    engine.register_suggestor(ApplicationIngestionAgent);
    engine.register_suggestor(DocumentVerificationAgent);
    engine.register_suggestor(CreditCheckAgent);
    engine.register_suggestor(ComplianceAgent);
    engine.register_suggestor(RiskAssessmentAgent);
    engine.register_suggestor(LoanDecisionAgent);

    let hitl_policy = EngineHitlPolicy {
        confidence_threshold: Some(0.75),
        gated_keys: vec![ContextKey::Proposals],
        timeout: TimeoutPolicy {
            timeout_secs: 300,
            action: TimeoutAction::Reject,
        },
    };
    engine.set_hitl_policy(hitl_policy);

    let application = serde_json::json!({
        "applicant": "Jane Smith",
        "requested_amount": 250000,
        "credit_score": 720,
        "income": 85000,
        "documents": true,
        "us_citizen": true,
        "age": 35,
        "bankruptcies": 0,
        "employment_years": 5
    });

    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "application-1", application.to_string());

    println!(
        "Processing loan application for ${}, credit score: {}\n",
        application["requested_amount"], application["credit_score"]
    );

    match engine.run_with_hitl(ctx).await {
        RunResult::HitlPause(pause) => {
            println!("⏸️  HITL Gate: Review Required");
            println!("    Decision needed for borderline application");

            if let Ok(proposal) = serde_json::from_str::<serde_json::Value>(&pause.request.summary)
            {
                let decision = proposal
                    .get("decision")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let score = proposal
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                println!("    Proposed: {} (score: {:.2})", decision, score);
            }
            println!();

            let decision =
                GateDecision::approve(pause.request.gate_id.clone(), "loan-officer@bank.com");

            println!("▶️  Approved by loan officer. Finalizing...\n");

            match engine.resume(*pause, decision).await {
                RunResult::Complete(Ok(result)) => {
                    for fact in result.context.get(ContextKey::Proposals) {
                        if let Ok(p) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                            let decision =
                                p.get("decision").and_then(|v| v.as_str()).unwrap_or("?");
                            let score = p.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            println!("✅ Loan {}! (score: {:.2})", decision, score);
                        }
                    }
                }
                _ => println!("❌ Decision failed"),
            }
        }
        RunResult::Complete(Ok(result)) => {
            for fact in result.context.get(ContextKey::Proposals) {
                if let Ok(p) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let decision = p.get("decision").and_then(|v| v.as_str()).unwrap_or("?");
                    let score = p.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let confidence = p.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    if confidence >= 0.75 {
                        println!(
                            "✅ Loan {}! (score: {:.2}, confidence: {:.0}%)",
                            decision,
                            score,
                            confidence * 100.0
                        );
                    } else {
                        println!(
                            "⏸️  Loan {} - requires review (score: {:.2}, confidence: {:.0}%)",
                            decision,
                            score,
                            confidence * 100.0
                        );
                    }
                }
            }
        }
        RunResult::Complete(Err(e)) => {
            println!("❌ Decision failed: {e}");
        }
    }

    println!("\n=== Done ===");
}
