use converge_core::{ContextKey, Invariant, InvariantClass, InvariantResult, Violation};

/// Invariant: Access must be explicitly granted by an authority.
/// Derived from @authority_required in trust.feature.
pub struct AuthorityRequired;

impl Invariant for AuthorityRequired {
    fn name(&self) -> &str {
        "authority_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        // Simplified check for the 15 jobs alignment:
        // "Given an AccessFact exists, Then a corresponding AuthorityDecision must exist in history"

        // We'll look for facts in any category that contain "access:granted"
        for key in [
            ContextKey::Signals,
            ContextKey::Strategies,
            ContextKey::Hypotheses,
        ] {
            for fact in ctx.get(key) {
                if fact.content.contains("access:granted") {
                    let has_authority = ctx.get(ContextKey::Signals).iter().any(|f| {
                        f.content.contains("authority:approved")
                            && f.content.contains(fact.id.as_str())
                    });

                    if !has_authority {
                        return InvariantResult::Violated(Violation::with_facts(
                            format!("Access grant {} lacks explicit authority approval", fact.id),
                            vec![fact.id.clone()],
                        ));
                    }
                }
            }
        }

        InvariantResult::Ok
    }
}

/// Invariant: Every transaction must have provenance.
/// Derived from @audit_trail_required in money.feature.
pub struct AuditTrailRequired;

impl Invariant for AuditTrailRequired {
    fn name(&self) -> &str {
        "audit_trail_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        // In a real system, we'd check the internal 'Fact' metadata.
        // For this MVP, we'll check if the content contains a 'provenance:' or 'by:' tag.

        for key in [ContextKey::Strategies, ContextKey::Evaluations] {
            for fact in ctx.get(key) {
                if !fact.content.contains("provenance:") && !fact.content.contains("by:") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Fact {} is missing required provenance metadata", fact.id),
                        vec![fact.id.clone()],
                    ));
                }
            }
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Fact;
    use std::collections::HashMap;

    struct FakeCtx(HashMap<ContextKey, Vec<Fact>>);

    impl converge_core::Context for FakeCtx {
        fn has(&self, key: ContextKey) -> bool {
            self.0.get(&key).is_some_and(|v| !v.is_empty())
        }
        fn get(&self, key: ContextKey) -> &[Fact] {
            self.0.get(&key).map_or(&[], Vec::as_slice)
        }
    }

    #[test]
    fn authority_required_name_and_class() {
        let inv = AuthorityRequired;
        assert_eq!(inv.name(), "authority_required");
        assert_eq!(inv.class(), InvariantClass::Semantic);
    }

    #[test]
    fn authority_required_passes_with_empty_ctx() {
        let inv = AuthorityRequired;
        let ctx = FakeCtx(HashMap::new());
        assert_eq!(inv.check(&ctx), InvariantResult::Ok);
    }

    #[test]
    fn audit_trail_required_name_and_class() {
        let inv = AuditTrailRequired;
        assert_eq!(inv.name(), "audit_trail_required");
        assert_eq!(inv.class(), InvariantClass::Structural);
    }

    #[test]
    fn audit_trail_required_passes_with_empty_ctx() {
        let inv = AuditTrailRequired;
        let ctx = FakeCtx(HashMap::new());
        assert_eq!(inv.check(&ctx), InvariantResult::Ok);
    }
}
