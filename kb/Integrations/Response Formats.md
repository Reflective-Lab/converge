---
tags: [integrations, llm, formats]
source: mixed
---
# Response Formats

Converge supports five output formats via `ResponseFormat`. The default for structured output is **YAML** — 10% fewer tokens than JSON, 100% model compliance, human-reviewable.

## Format overview

| Format   | Best for                                        | Token cost | Compliance | Parse |
|----------|-------------------------------------------------|------------|------------|-------|
| Yaml     | **Default for structured data.** Machine + human| Low (-10%) | 100%       | Yes   |
| Json     | API boundaries, storage, schema-ready payloads  | Medium     | 100%       | Yes   |
| Toml     | Flat config, simple records, settings           | Lowest     | 99%        | Yes   |
| Markdown | Human-facing reports, decision briefs           | Low        | 99%        | Opt.  |
| Text     | Free-form prose, no structure needed            | Lowest     | 100%       | No    |

## Choosing a format

**Start with YAML** for anything a machine will parse. It's cheaper than JSON, just as reliable, and a human can read and edit it. Fall back to JSON when the consumer requires it, when you want the strongest provider-native support, or when the next step needs schema validation outside the provider.

**Use JSON** at API boundaries, when storing to disk/DB, or when a downstream system mandates it. JSON is the universal interchange format and has native API enforcement on OpenAI and Gemini.

**Use TOML** only for flat key-value data (config, settings, simple records). TOML breaks on deeply nested structures — GPT-4o-mini produced invalid TOML for a nested audit trail in testing.

**Use Markdown** when a human reads the output (decision briefs, operator UIs, reports). Not a data interchange format. Some models return JSON in code fences when they interpret "structured" as "serialized" — the system instruction now explicitly asks for headings and tables.

**Use Text** when you genuinely want free-form output.

## Format vs schema

Converge now enforces **format**, not **schema**, at the provider boundary.

- `Json`, `Yaml`, and `Toml` responses are validated before the backend returns them.
- Trivial outer code fences are stripped for those machine formats.
- A chatty prose wrapper now fails with `LlmError::ResponseFormatMismatch`.
- Exact object shape is still the caller's responsibility.

If you need `"facts": [...]` specifically, or a fixed typed object shape, treat `ResponseFormat` as the transport contract and run a second schema validation step after parsing.

## Tested results (2026-04-14)

139/140 cells passed. 7 models × 5 formats × 4 prompts via OpenRouter.

### Compliance

| Model                    | Text | Markdown | Json | Yaml | Toml |
|--------------------------|------|----------|------|------|------|
| Claude Sonnet 4          | OK   | OK       | OK   | OK   | OK   |
| Claude Haiku 4.5         | OK   | OK       | OK   | OK   | OK   |
| GPT-4o                   | OK   | OK       | OK   | OK   | OK   |
| GPT-4o-mini              | OK   | OK       | OK   | OK   | FAIL |
| Gemini 2.5 Flash         | OK   | OK       | OK   | OK   | OK   |
| Llama 3.1 70B            | OK   | OK       | OK   | OK   | OK   |
| Mistral Large            | OK   | OK       | OK   | OK   | OK   |

One failure: GPT-4o-mini TOML on deeply nested data (`[action] = "value"` — invalid table header).

### Token efficiency

| Format | Avg input | Avg output | Avg total | vs JSON |
|--------|-----------|------------|-----------|---------|
| Yaml   | 131       | 152        | 282       | -10%    |
| Toml   | 136       | 167        | 303       | -3%     |
| Json   | 116       | 197        | 314       | baseline|
| Text   | 101       | 246        | 347       | +11%    |
| Md     | 147       | 372        | 518       | +65%    |

YAML saves ~10% on tokens. At scale this compounds because output tokens cost 3-5x more than input tokens.

### Latency

| Model              | Avg   | Min  | Max   |
|--------------------|-------|------|-------|
| Gemini 2.5 Flash   | 2.0s  | 0.7s | 8.6s  |
| GPT-4o             | 2.3s  | 0.7s | 4.3s  |
| GPT-4o-mini        | 2.7s  | 1.0s | 6.4s  |
| Claude Sonnet 4    | 3.6s  | 1.4s | 6.4s  |
| Claude Haiku 4.5   | 3.8s  | 1.0s | 10.3s |
| Mistral Large      | 4.3s  | 1.2s | 9.7s  |
| Llama 3.1 70B      | 8.9s  | 1.7s | 21.3s |

### Cost per 1000 calls

| Model              | Json   | Yaml   | Toml   | Md     |
|--------------------|--------|--------|--------|--------|
| GPT-4o-mini        | $0.12  | $0.09  | $0.08  | $0.14  |
| Gemini 2.5 Flash   | $0.14  | $0.11  | $0.12  | $0.68  |
| Llama 3.1 70B      | $0.17  | $0.15  | $0.17  | $0.20  |
| Claude Haiku 4.5   | $1.15  | $0.94  | $1.08  | $1.49  |
| Mistral Large      | $1.46  | $1.23  | $1.51  | $2.02  |
| GPT-4o             | $1.93  | $1.60  | $1.65  | $2.47  |
| Claude Sonnet 4    | $3.70  | $2.94  | $3.03  | $4.95  |

## Default strategy

Converge uses `ResponseFormat::default_structured()` → **YAML** for all structured output.

The selection system prefers the cheapest model that satisfies the task requirements. For structured extraction, that's typically GPT-4o-mini or Gemini 2.5 Flash (~$0.10/1000 calls).

On failure, the system retries with:
1. Same model, JSON format (native API enforcement)
2. Fallback model if the first model errors

## Running the matrix test

```bash
cd ~/dev/extensions/manifold
OPENROUTER_API_KEY=sk-or-v1-... cargo run --features openrouter --example format_matrix
```

See also: [[MCP Tools]], [[External Services]]
