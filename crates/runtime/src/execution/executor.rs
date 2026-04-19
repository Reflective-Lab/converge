// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Job executor for Converge Runtime.
//!
//! Provides a builder pattern for configuring and running convergence jobs.

use converge_core::suggestors::SeedSuggestor;
use converge_core::{Budget, Context, ContextKey, ConvergeResult, Engine, ExperienceStore};
use converge_experience::{InMemoryExperienceStore, StoreObserver};
use serde::Serialize;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tracing::{Instrument, info, info_span};

use crate::error::RuntimeError;
use crate::templates::{PackConfig, SeedFact};

use super::packs::{LlmConfig, register_pack_agents};
use super::streaming::{EventReceiver, RuntimeStreamingCallback};

/// Result of job execution.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    /// Whether convergence was reached.
    pub converged: bool,
    /// Number of cycles executed.
    pub cycles: u32,
    /// Final context.
    pub context: Context,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Fact counts by key.
    pub fact_counts: std::collections::HashMap<String, usize>,
    /// Integrity proof for the converged context.
    pub integrity: converge_core::IntegrityProof,
}

impl From<(ConvergeResult, u64)> for ExecutionResult {
    fn from((result, duration_ms): (ConvergeResult, u64)) -> Self {
        let fact_counts: std::collections::HashMap<String, usize> = ContextKey::iter()
            .map(|key| {
                let count = result.context.get(key).len();
                (format!("{key:?}"), count)
            })
            .collect();

        Self {
            converged: result.converged,
            cycles: result.cycles,
            context: result.context,
            duration_ms,
            fact_counts,
            integrity: result.integrity,
        }
    }
}

/// Builder for configuring and executing convergence jobs.
pub struct JobExecutorBuilder {
    /// Pack ID to use.
    pack_id: Option<String>,
    /// Pack configuration.
    pack_config: Option<PackConfig>,
    /// Seed facts.
    seeds: Vec<SeedFact>,
    /// Budget configuration.
    budget: Option<Budget>,
    /// LLM configuration.
    llm_config: LlmConfig,
    /// Streaming enabled.
    streaming_buffer: Option<usize>,
}

impl Default for JobExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JobExecutorBuilder {
    /// Create a new executor builder.
    pub fn new() -> Self {
        Self {
            pack_id: None,
            pack_config: None,
            seeds: Vec::new(),
            budget: None,
            llm_config: LlmConfig::default(),
            streaming_buffer: None,
        }
    }

    /// Set the pack ID to use.
    pub fn with_pack(mut self, pack_id: impl Into<String>) -> Self {
        self.pack_id = Some(pack_id.into());
        self
    }

    /// Set the pack configuration directly.
    pub fn with_pack_config(mut self, config: PackConfig) -> Self {
        self.pack_config = Some(config);
        self
    }

    /// Add seed facts.
    pub fn with_seeds(mut self, seeds: Vec<SeedFact>) -> Self {
        self.seeds = seeds;
        self
    }

    /// Add a single seed fact.
    pub fn with_seed(mut self, id: impl Into<String>, content: impl Into<String>) -> Self {
        self.seeds.push(SeedFact {
            id: id.into(),
            content: content.into(),
        });
        self
    }

    /// Set the budget.
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Set max cycles budget.
    pub fn with_max_cycles(mut self, max_cycles: u32) -> Self {
        let budget = self.budget.get_or_insert_with(Budget::default);
        budget.max_cycles = max_cycles;
        self
    }

    /// Set max facts budget.
    pub fn with_max_facts(mut self, max_facts: u32) -> Self {
        let budget = self.budget.get_or_insert_with(Budget::default);
        budget.max_facts = max_facts;
        self
    }

    /// Use mock LLM provider (deterministic, no API calls).
    ///
    /// This is useful for testing and development.
    pub fn with_mock_llm(mut self) -> Self {
        self.llm_config.use_mock = true;
        self
    }

    /// Use real LLM providers based on available API keys.
    ///
    /// This will use the `ProviderRegistry` to select the best available
    /// provider (Anthropic, OpenAI, etc.) based on environment variables.
    ///
    /// Falls back to deterministic agents if no API keys are available.
    pub fn with_real_llm(mut self) -> Self {
        self.llm_config.use_mock = false;
        self
    }

    /// Set LLM configuration.
    pub fn with_llm_config(mut self, config: LlmConfig) -> Self {
        self.llm_config = config;
        self
    }

    /// Enable streaming with the given buffer size.
    pub fn with_streaming(mut self, buffer_size: usize) -> Self {
        self.streaming_buffer = Some(buffer_size);
        self
    }

    /// Build the executor.
    pub fn build(self) -> Result<JobExecutor, RuntimeError> {
        let pack_id = self
            .pack_id
            .or_else(|| self.pack_config.as_ref().map(|c| c.name.clone()))
            .ok_or_else(|| RuntimeError::Config("No pack specified".to_string()))?;

        Ok(JobExecutor {
            pack_id,
            pack_config: self.pack_config,
            seeds: self.seeds,
            budget: self.budget.unwrap_or_default(),
            llm_config: self.llm_config,
            streaming_buffer: self.streaming_buffer,
        })
    }

    /// Build and execute immediately.
    pub async fn execute(self) -> Result<ExecutionResult, RuntimeError> {
        let executor = self.build()?;
        executor.run().await
    }

    /// Build and execute with streaming, returning the task handle and event receiver.
    pub fn execute_with_streaming(
        self,
    ) -> Result<
        (
            tokio::task::JoinHandle<Result<ExecutionResult, RuntimeError>>,
            EventReceiver,
        ),
        RuntimeError,
    > {
        let buffer_size = self.streaming_buffer.unwrap_or(256);
        let executor = self.with_streaming(buffer_size).build()?;
        executor.run_with_streaming()
    }
}

/// Job executor that runs convergence jobs.
pub struct JobExecutor {
    pack_id: String,
    pack_config: Option<PackConfig>,
    seeds: Vec<SeedFact>,
    budget: Budget,
    llm_config: LlmConfig,
    streaming_buffer: Option<usize>,
}

impl JobExecutor {
    /// Create a new executor builder.
    pub fn builder() -> JobExecutorBuilder {
        JobExecutorBuilder::new()
    }

    /// Run the job.
    pub async fn run(self) -> Result<ExecutionResult, RuntimeError> {
        let span = info_span!("job_execution", pack = %self.pack_id);
        async move {
            let start = std::time::Instant::now();

            info!(
                pack = %self.pack_id,
                seeds = self.seeds.len(),
                max_cycles = self.budget.max_cycles,
                "Starting job execution"
            );

            // Create engine with budget and event observer
            let mut engine = Engine::with_budget(self.budget);
            let experience_store = Arc::new(InMemoryExperienceStore::new());
            engine.set_event_observer(Arc::new(StoreObserver::new(experience_store.clone())));

            // Register seed agents
            for seed in &self.seeds {
                engine.register_suggestor(SeedSuggestor::new(&seed.id, &seed.content));
            }

            // Register pack agents
            register_pack_agents(&mut engine, &self.pack_id, &self.llm_config)?;

            // Create context and run
            let context = Context::new();
            let result = engine.run(context).await.map_err(RuntimeError::Converge)?;

            let duration_ms = start.elapsed().as_millis() as u64;
            let event_count = experience_store
                .query_events(&converge_core::EventQuery::default())
                .map(|e| e.len())
                .unwrap_or(0);

            info!(
                converged = result.converged,
                cycles = result.cycles,
                duration_ms = duration_ms,
                experience_events = event_count,
                "Job execution completed"
            );

            Ok(ExecutionResult::from((result, duration_ms)))
        }
        .instrument(span)
        .await
    }

    /// Run the job with streaming enabled.
    ///
    /// Returns a handle to the background task and an event receiver.
    pub fn run_with_streaming(
        self,
    ) -> Result<
        (
            tokio::task::JoinHandle<Result<ExecutionResult, RuntimeError>>,
            EventReceiver,
        ),
        RuntimeError,
    > {
        let buffer_size = self.streaming_buffer.unwrap_or(256);
        let (callback, receiver) = RuntimeStreamingCallback::channel(buffer_size);

        let span = info_span!("job_execution_streaming", pack = %self.pack_id);
        let handle = tokio::spawn(
            async move {
                let start = std::time::Instant::now();

                info!(
                    pack = %self.pack_id,
                    seeds = self.seeds.len(),
                    max_cycles = self.budget.max_cycles,
                    "Starting streaming job execution"
                );

                // Create engine with budget
                let mut engine = Engine::with_budget(self.budget);

                // Register seed agents
                for seed in &self.seeds {
                    engine.register_suggestor(SeedSuggestor::new(&seed.id, &seed.content));
                }

                // Register pack agents
                register_pack_agents(&mut engine, &self.pack_id, &self.llm_config)?;

                // Set streaming callback
                engine.set_streaming(callback.clone());

                // Create context and run
                let context = Context::new();
                let result = engine.run(context).await;

                let duration_ms = start.elapsed().as_millis() as u64;

                match result {
                    Ok(result) => {
                        // Emit final status
                        let total_facts = callback.fact_count();
                        callback.emit_converged(result.cycles, total_facts);

                        info!(
                            converged = result.converged,
                            cycles = result.cycles,
                            duration_ms = duration_ms,
                            "Streaming job execution completed"
                        );

                        Ok(ExecutionResult::from((result, duration_ms)))
                    }
                    Err(e) => {
                        callback.emit_halted(0, e.to_string());
                        Err(RuntimeError::Converge(e))
                    }
                }
            }
            .instrument(span),
        );

        Ok((handle, receiver))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_builder_requires_pack() {
        let result = JobExecutorBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_builder_with_pack() {
        let executor = JobExecutorBuilder::new()
            .with_pack("growth-strategy")
            .with_mock_llm()
            .build()
            .unwrap();

        assert_eq!(executor.pack_id, "growth-strategy");
    }

    #[test]
    fn test_executor_builder_with_seeds() {
        let executor = JobExecutorBuilder::new()
            .with_pack("growth-strategy")
            .with_seed("market:nordic", "Nordic B2B market")
            .with_seed("product:saas", "SaaS platform")
            .with_mock_llm()
            .build()
            .unwrap();

        assert_eq!(executor.seeds.len(), 2);
    }

    #[test]
    fn test_executor_builder_with_budget() {
        let executor = JobExecutorBuilder::new()
            .with_pack("growth-strategy")
            .with_max_cycles(100)
            .with_max_facts(500)
            .with_mock_llm()
            .build()
            .unwrap();

        assert_eq!(executor.budget.max_cycles, 100);
        assert_eq!(executor.budget.max_facts, 500);
    }

    #[test]
    fn test_execution_result_from_converge_result() {
        let mut context = Context::new();
        let _ = context.add_input(ContextKey::Seeds, "test", "content");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let context = runtime
            .block_on(Engine::new().run(context))
            .unwrap()
            .context;

        let converge_result = ConvergeResult {
            converged: true,
            cycles: 5,
            context,
            stop_reason: converge_core::StopReason::Converged,
            criteria_outcomes: Vec::new(),
            integrity: converge_core::integrity::IntegrityProof {
                merkle_root: converge_core::integrity::MerkleRoot::compute(&[]),
                clock_time: 0,
                fact_count: 0,
            },
        };

        let result = ExecutionResult::from((converge_result, 100));

        assert!(result.converged);
        assert_eq!(result.cycles, 5);
        assert_eq!(result.duration_ms, 100);
        assert!(result.fact_counts.contains_key("Seeds"));
    }
}
