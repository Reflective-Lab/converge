// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Typed admission boundary for external observations.
//!
//! Admission is not fact construction. External systems submit observations
//! with actor and provenance; Converge stages them as proposals so the normal
//! promotion gate remains the only path to governed facts.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::context::{ContextKey, ProposalId, ProposedFact};
use crate::types::{ActorId, ContentHash, TruthId};

/// Error raised while constructing an admission request.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdmissionError {
    /// Actor identity must be present.
    #[error("admission actor id must not be empty")]
    EmptyActorId,
    /// Observation source must be present.
    #[error("admission source must not be empty")]
    EmptySource,
    /// Proposal/idempotency key must be present.
    #[error("admission id must not be empty")]
    EmptyAdmissionId,
    /// Observation content must contain substantive payload.
    #[error("admission content must not be empty")]
    EmptyContent,
}

/// Actor class for an externally admitted observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionActorKind {
    /// Human user.
    Human,
    /// Automated agent.
    Agent,
    /// System component.
    System,
    /// External service or integration.
    External,
}

impl AdmissionActorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::System => "system",
            Self::External => "external",
        }
    }
}

/// Required actor metadata for an admission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionActor {
    id: ActorId,
    kind: AdmissionActorKind,
}

impl AdmissionActor {
    /// Creates a validated admission actor.
    pub fn new(id: impl Into<ActorId>, kind: AdmissionActorKind) -> Result<Self, AdmissionError> {
        let id = id.into();
        if id.as_str().trim().is_empty() {
            return Err(AdmissionError::EmptyActorId);
        }
        Ok(Self { id, kind })
    }

    /// Returns the actor identifier.
    #[must_use]
    pub fn id(&self) -> &ActorId {
        &self.id
    }

    /// Returns the actor kind.
    #[must_use]
    pub fn kind(&self) -> AdmissionActorKind {
        self.kind
    }
}

/// Validated source label for admission provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionSource(String);

impl AdmissionSource {
    /// Creates a validated source label.
    pub fn new(value: impl Into<String>) -> Result<Self, AdmissionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AdmissionError::EmptySource);
        }
        Ok(Self(value))
    }

    /// Borrows the source label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated observation payload for admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionContent(String);

impl AdmissionContent {
    /// Creates a validated observation payload.
    pub fn new(value: impl Into<String>) -> Result<Self, AdmissionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AdmissionError::EmptyContent);
        }
        Ok(Self(value))
    }

    /// Borrows the content payload.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn into_string(self) -> String {
        self.0
    }
}

/// Request to admit an external observation into the Converge truth pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    actor: AdmissionActor,
    source: AdmissionSource,
    key: ContextKey,
    id: ProposalId,
    content: AdmissionContent,
    target_truth_id: Option<TruthId>,
}

impl AdmissionRequest {
    /// Creates a validated admission request.
    pub fn new(
        actor: AdmissionActor,
        source: AdmissionSource,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: AdmissionContent,
    ) -> Result<Self, AdmissionError> {
        let id = id.into();
        if id.as_str().trim().is_empty() {
            return Err(AdmissionError::EmptyAdmissionId);
        }

        Ok(Self {
            actor,
            source,
            key,
            id,
            content,
            target_truth_id: None,
        })
    }

    /// Attaches the semantic truth target this observation is intended to inform.
    #[must_use]
    pub fn with_target_truth(mut self, truth_id: impl Into<TruthId>) -> Self {
        self.target_truth_id = Some(truth_id.into());
        self
    }

    /// Returns the target context key.
    #[must_use]
    pub fn key(&self) -> ContextKey {
        self.key
    }

    /// Returns the admission/proposal identifier.
    #[must_use]
    pub fn id(&self) -> &ProposalId {
        &self.id
    }

    /// Returns the content payload.
    #[must_use]
    pub fn content(&self) -> &AdmissionContent {
        &self.content
    }

    /// Returns the actor.
    #[must_use]
    pub fn actor(&self) -> &AdmissionActor {
        &self.actor
    }

    /// Returns the source label.
    #[must_use]
    pub fn source(&self) -> &AdmissionSource {
        &self.source
    }

    /// Returns the optional semantic target.
    #[must_use]
    pub fn target_truth_id(&self) -> Option<&TruthId> {
        self.target_truth_id.as_ref()
    }

    pub(crate) fn into_proposal(self) -> ProposedFact {
        let provenance = self.provenance();
        ProposedFact::new(self.key, self.id, self.content.into_string(), provenance)
    }

    fn provenance(&self) -> String {
        match &self.target_truth_id {
            Some(truth_id) => format!(
                "admission:{}:{}:{}:truth:{}",
                self.actor.kind.as_str(),
                self.actor.id,
                self.source.as_str(),
                truth_id
            ),
            None => format!(
                "admission:{}:{}:{}",
                self.actor.kind.as_str(),
                self.actor.id,
                self.source.as_str()
            ),
        }
    }
}

/// Receipt returned after staging an admitted observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    key: ContextKey,
    proposal_id: ProposalId,
    content_hash: ContentHash,
    target_truth_id: Option<TruthId>,
    staged: bool,
}

impl AdmissionReceipt {
    pub(crate) fn new(request: &AdmissionRequest, staged: bool) -> Self {
        Self {
            key: request.key,
            proposal_id: request.id.clone(),
            content_hash: content_hash(request.content.as_str()),
            target_truth_id: request.target_truth_id.clone(),
            staged,
        }
    }

    /// Returns the target context key.
    #[must_use]
    pub fn key(&self) -> ContextKey {
        self.key
    }

    /// Returns the staged proposal identifier.
    #[must_use]
    pub fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }

    /// Returns the content hash for audit/idempotency checks.
    #[must_use]
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    /// Returns the optional semantic target.
    #[must_use]
    pub fn target_truth_id(&self) -> Option<&TruthId> {
        self.target_truth_id.as_ref()
    }

    /// Returns true if this call staged a new proposal.
    #[must_use]
    pub fn staged(&self) -> bool {
        self.staged
    }
}

fn content_hash(content: &str) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    ContentHash::new(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextState;
    use converge_pack::Context as _;

    fn actor() -> AdmissionActor {
        AdmissionActor::new("organism-runtime", AdmissionActorKind::System).unwrap()
    }

    fn source() -> AdmissionSource {
        AdmissionSource::new("truth-document").unwrap()
    }

    fn content(value: &str) -> AdmissionContent {
        AdmissionContent::new(value).unwrap()
    }

    #[test]
    fn admission_requires_actor_source_id_and_content() {
        assert_eq!(
            AdmissionActor::new("", AdmissionActorKind::System),
            Err(AdmissionError::EmptyActorId)
        );
        assert_eq!(AdmissionSource::new(" "), Err(AdmissionError::EmptySource));
        assert!(matches!(
            AdmissionRequest::new(
                actor(),
                source(),
                ContextKey::Seeds,
                "",
                content("claim payload")
            ),
            Err(AdmissionError::EmptyAdmissionId)
        ));
        assert_eq!(
            AdmissionContent::new(" "),
            Err(AdmissionError::EmptyContent)
        );
    }

    #[test]
    fn admission_stages_proposal_not_fact() {
        let mut context = ContextState::new();
        let request = AdmissionRequest::new(
            actor(),
            source(),
            ContextKey::Seeds,
            "truth-doc-1",
            content(r#"{"claim":"approved source"}"#),
        )
        .unwrap()
        .with_target_truth("truth-1");

        let receipt = context.submit_observation(request).unwrap();

        assert!(receipt.staged());
        assert_eq!(receipt.key(), ContextKey::Seeds);
        assert_eq!(receipt.proposal_id().as_str(), "truth-doc-1");
        assert_eq!(
            receipt.target_truth_id().map(TruthId::as_str),
            Some("truth-1")
        );
        assert!(!context.has(ContextKey::Seeds));
        assert_eq!(context.get_proposals(ContextKey::Seeds).len(), 1);
    }

    #[test]
    fn duplicate_admission_is_idempotent_when_payload_matches() {
        let mut context = ContextState::new();
        let request = AdmissionRequest::new(
            actor(),
            source(),
            ContextKey::Seeds,
            "truth-doc-1",
            content("same payload"),
        )
        .unwrap();

        let first = context.submit_observation(request.clone()).unwrap();
        let second = context.submit_observation(request).unwrap();

        assert!(first.staged());
        assert!(!second.staged());
        assert_eq!(first.content_hash(), second.content_hash());
        assert_eq!(context.get_proposals(ContextKey::Seeds).len(), 1);
    }

    #[test]
    fn duplicate_admission_rejects_conflicting_payload() {
        let mut context = ContextState::new();
        let first = AdmissionRequest::new(
            actor(),
            source(),
            ContextKey::Seeds,
            "truth-doc-1",
            content("first payload"),
        )
        .unwrap();
        let second = AdmissionRequest::new(
            actor(),
            source(),
            ContextKey::Seeds,
            "truth-doc-1",
            content("second payload"),
        )
        .unwrap();

        context.submit_observation(first).unwrap();
        let err = context.submit_observation(second).unwrap_err();

        assert!(matches!(err, crate::ConvergeError::Conflict { .. }));
        assert_eq!(context.get_proposals(ContextKey::Seeds).len(), 1);
    }
}
