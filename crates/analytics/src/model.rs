// Copyright 2024-2026 Reflective Labs

use crate::engine::FeatureVector;
use burn::{
    nn::{Linear, LinearConfig, Relu},
    prelude::*,
    tensor::{Tensor, backend::Backend},
};
use converge_core::{AgentEffect, ContextKey, ProposedFact, Suggestor};

// Re-defining for now if not public in engine, strictly we should move to lib or common
// But for this example we assume we can deserialize into this struct.

/// Simple MLP Model
#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
    activation: Relu,
}

impl<B: Backend> Model<B> {
    pub fn new(device: &B::Device) -> Self {
        // Initialize with default config for demo
        let config = ModelConfig::new(3, 16, 1);
        config.init(device)
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.fc1.forward(input);
        let x = self.activation.forward(x);
        self.fc2.forward(x)
    }
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
}

impl ModelConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            fc1: LinearConfig::new(self.input_size, self.hidden_size).init(device),
            fc2: LinearConfig::new(self.hidden_size, self.output_size).init(device),
            activation: Relu::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct InferenceAgent {
    // in real app, model might be Arc<Mutex<Model>> or just loaded
    // For demo we instantiate on fly or would hold it.
    // Burn models are cheap to clone if weights are Arc.
    // For this demo, we won't hold the model in the struct to avoid generic complexity in the Suggestor trait object,
    // or we use a concrete backend like NdArrayBackend.
}

impl InferenceAgent {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Suggestor for InferenceAgent {
    fn name(&self) -> &str {
        "InferenceAgent (Burn)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run if there are proposals (features) but no hypothesis yet
        ctx.has(ContextKey::Proposals) && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        // 1. Find the feature proposal
        // In reality, filtered by provenance "polars-engine"
        let _proposals = ctx.get(ContextKey::Proposals); // wait, ctx.get returns Fact, but proposals are ProposedFacts?
        // Ah, ctx.get(ContextKey) returns FACTs (promoted).
        // If FeatureAgent emits PROPOSALS, they are in `ContextKey::Proposals`?
        // Wait, ContextKey::Proposals is a key where Validated Proposals might live?
        // OR does FeatureAgent emit *Facts* directly if trusted?

        // In the `engine.rs` implementation I sent `ProposedFact` with key `ContextKey::Proposals`.
        // If they are not promoted to Facts, they are not in `ctx.get()`.
        // `Context` only stores `facts`.
        // Proposals usually sit in a queue in the Engine or are added to Context if Key::Proposals is a storage for them?
        // Looking at `ContextKey` definition: "Internal storage for proposed facts before validation."
        // So they ARE stored as FACTS under the key `Proposals` if the system works that way?
        // OR `ProposedFact`s are converted to `Fact`s by the engine.
        // `ProposedFact::try_from` converts to `Fact`.
        // If the engine accepts the proposal, it adds it as a Fact.

        // Let's assume the engine validated it and stored it.
        // So we look for Facts in `ContextKey::Proposals`?
        // Actually, normally `Proposals` key is for... proposals.
        // But `FeatureAgent` intended to propose `context.key = Proposals`?
        // No, `FeatureAgent` sent `proposal.key = Proposals`.

        // Let's assume we find the features in `ContextKey::Proposals` (as stored Facts).

        // We iterate and find one we haven't processed? For now just take the first.

        // This logic is simplified for demo.

        let facts = ctx.get(ContextKey::Proposals);
        if facts.is_empty() {
            return AgentEffect::empty();
        }

        let fact_content = &facts[0].content;

        // 2. Deserialize features
        let features: FeatureVector = match serde_json::from_str(fact_content) {
            Ok(f) => f,
            Err(_) => return AgentEffect::empty(),
        };

        // 3. Run Inference (Burn)
        type B = burn::backend::NdArray;
        let device = Default::default();
        let model: Model<B> = ModelConfig::new(3, 16, 1).init(&device);

        let input = Tensor::<B, 1>::from_floats(features.data.as_slice(), &device)
            .reshape([features.shape[0], features.shape[1]]);

        let output = model.forward(input);

        // 4. Emit Hypothesis
        let values: Vec<f32> = output.into_data().to_vec::<f32>().unwrap_or_default();
        let prediction = values[0]; // Assume single output

        let hypo_content = format!("Prediction: {:.4} (based on {})", prediction, facts[0].id);

        let hypothesis = ProposedFact::new(
            ContextKey::Hypotheses,
            format!("hypo-{}", facts[0].id),
            hypo_content,
            self.name(),
        );

        AgentEffect::with_proposal(hypothesis)
    }
}

/// Run batch inference on a [`FeatureVector`] using a configured model.
///
/// Abstracts Burn internals: the caller provides a [`ModelConfig`] and
/// a [`FeatureVector`] (shape [n, input_size]), and receives a `Vec<f32>`
/// of per-sample predictions.
///
/// Uses the `NdArray` backend internally.
pub fn run_batch_inference(
    config: &ModelConfig,
    features: &FeatureVector,
) -> anyhow::Result<Vec<f32>> {
    type B = burn::backend::NdArray;
    let device = Default::default();
    let model: Model<B> = config.init(&device);

    let n = features.rows();
    let input = Tensor::<B, 1>::from_floats(features.data.as_slice(), &device)
        .reshape([n, config.input_size]);
    let output = model.forward(input);
    let values: Vec<f32> = output.into_data().to_vec::<f32>().unwrap_or_default();
    Ok(values)
}
