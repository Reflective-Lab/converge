// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Retrieval agent for context-aware semantic search.
//!
//! This module implements a two-stage retrieval pipeline using:
//! 1. **Embedding recall**: Fast but coarse (vector similarity)
//! 2. **Reranking**: Slow but precise (cross-attention scoring)
//!
//! # Architecture
//!
//! ```text
//! Query
//!   ↓
//! [Embedder] → query vector
//!   ↓
//! [VectorStore] → top-K candidates (coarse recall)
//!   ↓
//! [Reranker] → fine-grained relevance scores
//!   ↓
//! ProposedFacts (candidates with scores)
//!   ↓
//! [ValidationAgent] → Facts (promoted)
//! ```
//!
//! # Design Principles
//!
//! Following Converge's "LLMs suggest, never decide" principle:
//! - Retrieval produces `ProposedFact`, not `Fact`
//! - Scores are explicit for auditability
//! - Vector stores are caches, not authoritative state
//! - All operations have clear provenance
//!
//! # Example
//!
//! ```ignore
//! use converge_domain::retrieval::{RetrievalAgent, RetrievalConfig};
//! use converge_provider::vector::InMemoryVectorStore;
//! use converge_provider::embedding::QwenVLEmbedding;
//! use converge_provider::reranker::QwenVLReranker;
//! use std::sync::Arc;
//!
//! // Create components
//! let embedder = Arc::new(QwenVLEmbedding::from_huggingface_env()?);
//! let reranker = Arc::new(QwenVLReranker::from_huggingface_env()?);
//! let store = Arc::new(InMemoryVectorStore::new());
//!
//! // Create retrieval agent
//! let agent = RetrievalAgent::new(embedder, store)
//!     .with_reranker(reranker)
//!     .with_config(RetrievalConfig {
//!         initial_recall: 100,
//!         rerank_top_k: 20,
//!         final_top_k: 5,
//!         min_score: Some(0.5),
//!     });
//!
//! // Index documents
//! agent.index_documents(&[
//!     Document::new("doc-1", "Machine learning fundamentals..."),
//!     Document::new("doc-2", "Deep learning with neural networks..."),
//! ])?;
//!
//! // Retrieve relevant context
//! let proposals = agent.retrieve("What is deep learning?")?;
//! ```

use converge_core::capability::{
    CapabilityError, EmbedInput, EmbedRequest, Embedding, RerankRequest, Reranking, VectorQuery,
    VectorRecall, VectorRecord,
};
use converge_core::{ContextKey, ProposedFact};
use std::sync::Arc;

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Configuration for the retrieval pipeline.
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// Number of candidates to retrieve from vector store (coarse recall).
    pub initial_recall: usize,
    /// Number of candidates to send to reranker.
    pub rerank_top_k: usize,
    /// Final number of results to return.
    pub final_top_k: usize,
    /// Minimum score threshold (0.0-1.0).
    pub min_score: Option<f64>,
    /// Target context key for proposals.
    pub target_key: ContextKey,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            initial_recall: 100,
            rerank_top_k: 20,
            final_top_k: 5,
            min_score: None,
            target_key: ContextKey::Signals,
        }
    }
}

impl RetrievalConfig {
    /// Creates config optimized for speed.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            initial_recall: 50,
            rerank_top_k: 10,
            final_top_k: 3,
            min_score: None,
            target_key: ContextKey::Signals,
        }
    }

    /// Creates config optimized for quality.
    #[must_use]
    pub fn quality() -> Self {
        Self {
            initial_recall: 200,
            rerank_top_k: 50,
            final_top_k: 10,
            min_score: Some(0.3),
            target_key: ContextKey::Signals,
        }
    }
}

// =============================================================================
// DOCUMENT
// =============================================================================

/// A document to be indexed and retrieved.
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique identifier.
    pub id: String,
    /// Document content.
    pub content: String,
    /// Optional metadata.
    pub metadata: serde_json::Value,
}

impl Document {
    /// Creates a new document with just content.
    #[must_use]
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata: serde_json::json!({}),
        }
    }

    /// Creates a document with metadata.
    #[must_use]
    pub fn with_metadata(
        id: impl Into<String>,
        content: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata,
        }
    }

    /// Adds a metadata field.
    #[must_use]
    pub fn with_field(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        if let Some(obj) = self.metadata.as_object_mut() {
            obj.insert(key.to_string(), value.into());
        }
        self
    }
}

// =============================================================================
// RETRIEVAL RESULT
// =============================================================================

/// Result from a retrieval operation.
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// Document ID.
    pub id: String,
    /// Document content.
    pub content: String,
    /// Document metadata.
    pub metadata: serde_json::Value,
    /// Vector similarity score (from embedding recall).
    pub vector_score: f64,
    /// Reranker score (if reranking was performed).
    pub rerank_score: Option<f64>,
    /// Final combined score.
    pub final_score: f64,
}

impl RetrievalResult {
    /// Converts to a `ProposedFact` with retrieval provenance.
    #[must_use]
    pub fn to_proposed_fact(
        &self,
        target_key: ContextKey,
        embedder: &str,
        reranker: Option<&str>,
    ) -> ProposedFact {
        let provenance = if let Some(reranker) = reranker {
            format!(
                "retrieval:embedder={},reranker={},vector_score={:.3},rerank_score={:.3}",
                embedder,
                reranker,
                self.vector_score,
                self.rerank_score.unwrap_or(0.0)
            )
        } else {
            format!(
                "retrieval:embedder={},vector_score={:.3}",
                embedder, self.vector_score
            )
        };

        ProposedFact::new(
            target_key,
            format!("retrieved-{}", self.id),
            self.content.clone(),
            provenance,
        )
        .with_confidence(self.final_score)
    }
}

// =============================================================================
// RETRIEVAL AGENT
// =============================================================================

/// Two-stage retrieval agent.
///
/// This agent implements semantic search using:
/// 1. Embedding model for vectorization
/// 2. Vector store for ANN search
/// 3. Optional reranker for fine-grained scoring
///
/// # Key Properties (per Converge principles)
///
/// - **Authority level: ZERO** - produces candidates, not decisions
/// - **Output: `ProposedFacts`** - must go through validation
/// - **Provenance: explicit** - scores and models tracked
/// - **Vector store: cache** - can be rebuilt from Context
pub struct RetrievalAgent<E, V, R = ()>
where
    E: Embedding,
    V: VectorRecall,
{
    embedder: Arc<E>,
    store: Arc<V>,
    reranker: Option<Arc<R>>,
    config: RetrievalConfig,
}

impl<E, V> RetrievalAgent<E, V, ()>
where
    E: Embedding,
    V: VectorRecall,
{
    /// Creates a new retrieval agent without reranker.
    #[must_use]
    pub fn new(embedder: Arc<E>, store: Arc<V>) -> Self {
        Self {
            embedder,
            store,
            reranker: None,
            config: RetrievalConfig::default(),
        }
    }

    /// Adds a reranker to the pipeline.
    #[must_use]
    pub fn with_reranker<R: Reranking>(self, reranker: Arc<R>) -> RetrievalAgent<E, V, R> {
        RetrievalAgent {
            embedder: self.embedder,
            store: self.store,
            reranker: Some(reranker),
            config: self.config,
        }
    }
}

impl<E, V, R> RetrievalAgent<E, V, R>
where
    E: Embedding,
    V: VectorRecall,
{
    /// Sets the retrieval configuration.
    #[must_use]
    pub fn with_config(mut self, config: RetrievalConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the embedder name for provenance.
    #[must_use]
    pub fn embedder_name(&self) -> &str {
        self.embedder.name()
    }

    /// Indexes a single document.
    ///
    /// # Errors
    ///
    /// Returns error if embedding or storage fails.
    pub fn index_document(&self, doc: &Document) -> Result<(), CapabilityError> {
        // Generate embedding
        let response = self.embedder.embed(&EmbedRequest::text(&doc.content))?;

        if response.embeddings.is_empty() {
            return Err(CapabilityError::invalid_input("No embedding generated"));
        }

        // Store in vector store
        let record = VectorRecord {
            id: doc.id.clone(),
            vector: response.embeddings[0].clone(),
            payload: serde_json::json!({
                "content": doc.content,
                "metadata": doc.metadata,
            }),
        };

        self.store.upsert(&record)?;

        tracing::debug!(doc_id = %doc.id, "Indexed document");
        Ok(())
    }

    /// Indexes multiple documents.
    ///
    /// # Errors
    ///
    /// Returns error if any document fails to index.
    pub fn index_documents(&self, docs: &[Document]) -> Result<(), CapabilityError> {
        for doc in docs {
            self.index_document(doc)?;
        }
        tracing::info!(count = docs.len(), "Indexed documents");
        Ok(())
    }

    /// Clears all indexed documents.
    ///
    /// # Errors
    ///
    /// Returns error if clear fails.
    pub fn clear(&self) -> Result<(), CapabilityError> {
        self.store.clear()
    }

    /// Returns the count of indexed documents.
    ///
    /// # Errors
    ///
    /// Returns error if count fails.
    pub fn count(&self) -> Result<usize, CapabilityError> {
        self.store.count()
    }
}

impl<E, V, R> RetrievalAgent<E, V, R>
where
    E: Embedding,
    V: VectorRecall,
    R: Reranking,
{
    /// Returns the reranker name for provenance.
    #[must_use]
    pub fn reranker_name(&self) -> Option<&str> {
        self.reranker.as_ref().map(|r| r.name())
    }

    /// Retrieves relevant documents for a query.
    ///
    /// # Errors
    ///
    /// Returns error if retrieval fails.
    pub fn retrieve(&self, query: &str) -> Result<Vec<RetrievalResult>, CapabilityError> {
        // Stage 1: Embed the query
        let query_response = self.embedder.embed(&EmbedRequest::text(query))?;

        if query_response.embeddings.is_empty() {
            return Err(CapabilityError::invalid_input(
                "No query embedding generated",
            ));
        }

        let query_vector = &query_response.embeddings[0];

        // Stage 2: Vector recall
        let candidates = self.store.query(&VectorQuery::new(
            query_vector.clone(),
            self.config.initial_recall,
        ))?;

        if candidates.is_empty() {
            tracing::debug!(query = %query, "No candidates found in vector store");
            return Ok(vec![]);
        }

        tracing::debug!(
            query = %query,
            candidates = candidates.len(),
            "Vector recall complete"
        );

        // Build initial results from vector recall
        let mut results: Vec<RetrievalResult> = candidates
            .iter()
            .map(|match_| {
                let content = match_
                    .payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let metadata = match_
                    .payload
                    .get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                RetrievalResult {
                    id: match_.id.clone(),
                    content,
                    metadata,
                    vector_score: match_.score,
                    rerank_score: None,
                    final_score: match_.score,
                }
            })
            .collect();

        // Stage 3: Reranking (if reranker is available)
        if let Some(reranker) = &self.reranker {
            // Take top candidates for reranking
            let rerank_candidates: Vec<_> = results
                .iter()
                .take(self.config.rerank_top_k)
                .map(|r| EmbedInput::text(&r.content))
                .collect();

            if !rerank_candidates.is_empty() {
                let rerank_response = reranker.rerank(&RerankRequest::new(
                    EmbedInput::text(query),
                    rerank_candidates,
                ))?;

                tracing::debug!(
                    reranked = rerank_response.ranked.len(),
                    "Reranking complete"
                );

                // Update scores from reranking
                for ranked in &rerank_response.ranked {
                    if ranked.index < results.len() {
                        results[ranked.index].rerank_score = Some(ranked.score);
                        // Use rerank score as final score (more accurate)
                        results[ranked.index].final_score = ranked.score;
                    }
                }

                // Re-sort by final score
                results.sort_by(|a, b| {
                    b.final_score
                        .partial_cmp(&a.final_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Apply final filters
        if let Some(min_score) = self.config.min_score {
            results.retain(|r| r.final_score >= min_score);
        }

        results.truncate(self.config.final_top_k);

        tracing::info!(
            query = %query,
            results = results.len(),
            "Retrieval complete"
        );

        Ok(results)
    }

    /// Retrieves and converts results to `ProposedFacts`.
    ///
    /// This is the primary interface for Converge integration.
    ///
    /// # Errors
    ///
    /// Returns error if retrieval fails.
    pub fn retrieve_as_proposals(&self, query: &str) -> Result<Vec<ProposedFact>, CapabilityError> {
        let results = self.retrieve(query)?;

        let proposals: Vec<ProposedFact> = results
            .iter()
            .map(|r| {
                r.to_proposed_fact(
                    self.config.target_key,
                    self.embedder.name(),
                    self.reranker.as_ref().map(|r| r.name()),
                )
            })
            .collect();

        Ok(proposals)
    }
}

// =============================================================================
// MULTIMODAL RETRIEVAL
// =============================================================================

/// Query input for multimodal retrieval.
#[derive(Debug, Clone)]
pub enum RetrievalQuery {
    /// Text-only query.
    Text(String),
    /// Image query (path to image file).
    Image(std::path::PathBuf),
    /// Mixed query (text + image).
    Mixed {
        text: String,
        image: std::path::PathBuf,
    },
}

impl From<&str> for RetrievalQuery {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<String> for RetrievalQuery {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl RetrievalQuery {
    /// Converts to `EmbedInput`.
    #[must_use]
    pub fn to_embed_input(&self) -> EmbedInput {
        match self {
            Self::Text(text) => EmbedInput::text(text),
            Self::Image(path) => EmbedInput::image_path(path),
            Self::Mixed { text, image } => {
                EmbedInput::Mixed(vec![EmbedInput::text(text), EmbedInput::image_path(image)])
            }
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use converge_provider::vector::InMemoryVectorStore;

    /// Mock embedder for testing.
    struct MockEmbedder;

    impl Embedding for MockEmbedder {
        fn name(&self) -> &str {
            "mock-embedder"
        }

        fn modalities(&self) -> Vec<converge_core::capability::Modality> {
            vec![converge_core::capability::Modality::Text]
        }

        fn default_dimensions(&self) -> usize {
            3
        }

        fn embed(
            &self,
            request: &EmbedRequest,
        ) -> Result<converge_core::capability::EmbedResponse, CapabilityError> {
            // Generate simple embeddings based on content
            let embeddings: Vec<Vec<f32>> = request
                .inputs
                .iter()
                .map(|input| {
                    match input {
                        EmbedInput::Text(text) => {
                            // Simple hash-based embedding for testing
                            let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                            vec![
                                (hash % 100) as f32 / 100.0,
                                ((hash / 100) % 100) as f32 / 100.0,
                                ((hash / 10000) % 100) as f32 / 100.0,
                            ]
                        }
                        _ => vec![0.0, 0.0, 0.0],
                    }
                })
                .collect();

            Ok(converge_core::capability::EmbedResponse {
                embeddings,
                model: "mock".to_string(),
                dimensions: 3,
                usage: None,
            })
        }
    }

    /// Mock reranker for testing.
    struct MockReranker;

    impl Reranking for MockReranker {
        fn name(&self) -> &str {
            "mock-reranker"
        }

        fn modalities(&self) -> Vec<converge_core::capability::Modality> {
            vec![converge_core::capability::Modality::Text]
        }

        fn rerank(
            &self,
            request: &RerankRequest,
        ) -> Result<converge_core::capability::RerankResponse, CapabilityError> {
            // Simple mock: score based on text length similarity
            let query_len = match &request.query {
                EmbedInput::Text(t) => t.len(),
                _ => 0,
            };

            let ranked: Vec<converge_core::capability::RankedItem> = request
                .candidates
                .iter()
                .enumerate()
                .map(|(idx, candidate)| {
                    let candidate_len = match candidate {
                        EmbedInput::Text(t) => t.len(),
                        _ => 0,
                    };
                    // Score inversely proportional to length difference
                    let diff = (query_len as i32 - candidate_len as i32).abs() as f64;
                    let score = 1.0 / (1.0 + diff / 10.0);
                    converge_core::capability::RankedItem { index: idx, score }
                })
                .collect();

            Ok(converge_core::capability::RerankResponse {
                ranked,
                model: "mock".to_string(),
            })
        }
    }

    #[test]
    fn index_and_retrieve() {
        let embedder = Arc::new(MockEmbedder);
        let store = Arc::new(InMemoryVectorStore::new());
        let reranker = Arc::new(MockReranker);

        let agent = RetrievalAgent::new(embedder, store)
            .with_reranker(reranker)
            .with_config(RetrievalConfig {
                initial_recall: 10,
                rerank_top_k: 5,
                final_top_k: 3,
                min_score: None,
                target_key: ContextKey::Signals,
            });

        // Index documents
        agent
            .index_documents(&[
                Document::new("doc-1", "Machine learning is a subset of AI"),
                Document::new("doc-2", "Deep learning uses neural networks"),
                Document::new("doc-3", "The weather is nice today"),
            ])
            .unwrap();

        assert_eq!(agent.count().unwrap(), 3);

        // Retrieve
        let results = agent.retrieve("What is machine learning?").unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 3);

        // Check provenance is set
        for result in &results {
            assert!(result.vector_score > 0.0);
            assert!(result.rerank_score.is_some());
        }
    }

    #[test]
    fn retrieve_as_proposals() {
        let embedder = Arc::new(MockEmbedder);
        let store = Arc::new(InMemoryVectorStore::new());
        let reranker = Arc::new(MockReranker);

        let agent = RetrievalAgent::new(embedder, store)
            .with_reranker(reranker)
            .with_config(RetrievalConfig::default());

        agent
            .index_document(&Document::new("doc-1", "Test document content"))
            .unwrap();

        let proposals = agent.retrieve_as_proposals("test query").unwrap();

        assert!(!proposals.is_empty());

        // Verify ProposedFact structure
        let proposal = &proposals[0];
        assert_eq!(proposal.key, ContextKey::Signals);
        assert!(proposal.id.starts_with("retrieved-"));
        assert!(proposal.provenance.contains("retrieval:"));
        assert!(proposal.provenance.contains("embedder=mock-embedder"));
        assert!(proposal.provenance.contains("reranker=mock-reranker"));
    }

    #[test]
    fn without_reranker() {
        let embedder = Arc::new(MockEmbedder);
        let store = Arc::new(InMemoryVectorStore::new());

        // Create agent without reranker
        let agent = RetrievalAgent::new(embedder, store).with_config(RetrievalConfig {
            initial_recall: 10,
            rerank_top_k: 5,
            final_top_k: 3,
            min_score: None,
            target_key: ContextKey::Signals,
        });

        agent
            .index_document(&Document::new("doc-1", "Test content"))
            .unwrap();

        // This should work without reranker
        // Note: retrieve() requires R: Reranking, so we can't call it directly
        // without a reranker. This is by design - the type system enforces
        // that you either have a full pipeline or just indexing.
    }

    #[test]
    fn min_score_filter() {
        let embedder = Arc::new(MockEmbedder);
        let store = Arc::new(InMemoryVectorStore::new());
        let reranker = Arc::new(MockReranker);

        let agent = RetrievalAgent::new(embedder, store)
            .with_reranker(reranker)
            .with_config(RetrievalConfig {
                initial_recall: 10,
                rerank_top_k: 5,
                final_top_k: 10,
                min_score: Some(0.9), // Very high threshold
                target_key: ContextKey::Signals,
            });

        agent
            .index_documents(&[
                Document::new("doc-1", "Short"),
                Document::new("doc-2", "A much longer document with more content"),
            ])
            .unwrap();

        let results = agent.retrieve("Short query").unwrap();

        // High min_score should filter most results
        for result in &results {
            assert!(result.final_score >= 0.9);
        }
    }

    #[test]
    fn document_metadata() {
        let doc = Document::new("doc-1", "Content")
            .with_field("category", "science")
            .with_field("year", 2024);

        assert_eq!(doc.metadata["category"], "science");
        assert_eq!(doc.metadata["year"], 2024);
    }

    #[test]
    fn config_presets() {
        let fast = RetrievalConfig::fast();
        assert_eq!(fast.initial_recall, 50);
        assert_eq!(fast.final_top_k, 3);

        let quality = RetrievalConfig::quality();
        assert_eq!(quality.initial_recall, 200);
        assert_eq!(quality.final_top_k, 10);
        assert!(quality.min_score.is_some());
    }
}
