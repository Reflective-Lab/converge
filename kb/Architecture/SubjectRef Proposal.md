# SubjectRef Proposal

Status: **proposal**, not implemented. Seeking maintainer alignment before code.
Originating context: Atlas integration app + Quorum / Warden / Movement cross-app citation work in `marquee-apps/atlas-integration/kb/Architecture/Upstream Types Audit.md`.

## TL;DR

Converge has typed references for **who** acts (`Actor`, `Provenance`) but no typed reference for **what** is being reasoned about. Apps cite "the candidate," "the question," "the gate," "the asset" today as opaque URI strings in `IntentPacket.context` and in fact payloads — `"atlas://acquisition-assets/shared-identity-core"`, `"quorum://unresolved-questions/identity-owner-coverage"`, `"warden://dd-gates/dd-evidence.identity-data-residency"`.

This proposal adds a typed `SubjectRef` to Converge so that:

- Fact promotion can tag facts by subject without strings-in-disguise
- Cross-app citation resolution becomes a typed boundary
- Helms readiness federation can ask "what's the status of *this subject*" without parsing URIs
- Apps stop drifting on URI shape

## Motivation

### What's broken today

The `Actor` type at `crates/core/src/types/provenance.rs:176` answers "who proposed this fact." There is no parallel typed answer to "what is this fact about." Apps fall back to one of three patterns, all weak:

1. **URI string in `IntentPacket.context`.** Atlas's `crates/atlas-app/src/intent.rs` writes `"subject_ref": "atlas://acquisition-assets/shared-identity-core"` as JSON. Typos are silent. Refactors leak.
2. **URI string in fact payload.** Atlas's evidence fixtures carry `"source_ref": "fixture://commercial-access/auth/jwt.py"` as a free-text field on each evidence record. Cross-app resolution against these strings is ad hoc.
3. **Hardcoded constant.** `pub const ACQUISITION_EVIDENCE_SUBJECT_REF: &str = "atlas://acquisition-assets/shared-identity-core";` — works as a one-off; breaks the moment a second app cites the same subject and they disagree on capitalization or path style.

The downstream pain shows up at three boundaries:

- **Cross-app citation resolver** (planned in `application-kernel` or as a `cross-app-citations` adapter) has no typed input. The resolver signature today would have to be `fn resolve(uri: &str) -> Result<Box<dyn Any>>`, which is exactly the shape that resists Cargo-level type checking.
- **Helms readiness federation** wants to ask "is the system ready to produce evidence for `atlas://acquisition-assets/shared-identity-core`?" Without a typed `SubjectRef`, Helms has to either ship a URI parser per app or accept stringly-typed queries.
- **Fact tagging.** A `ProposedFact` today carries `ContextKey` (the 12 closed-enum keys) and `Provenance` (who). It does not carry "what this fact is about." Subjects are reconstructed from payload fields, app by app, with no shared schema.

### Why now

Atlas's intent helper (`crates/atlas-app/src/intent.rs`) shipped using URI strings as a stepping stone. Quorum, Warden, and Movement will need to cite the same subjects. The "stepping stone" becomes load-bearing the moment a second app does cross-app citation. Catching this before it's the only path is cheaper than retrofitting later.

## Proposed type

New module at `crates/core/src/types/subject.rs`:

```rust
//! Typed reference to a subject being reasoned about.
//!
//! Complements `Actor` (who is reasoning) and `ContextKey` (where in the
//! engagement state a fact lives) with a typed answer to "what is this
//! fact / intent / citation about."

use serde::{Deserialize, Serialize};
use std::fmt;

/// A typed reference to a subject being reasoned about.
///
/// Three-part structure: `scheme` (the owning app), `kind` (the kind of
/// thing in that app's domain), `id` (the concrete instance).
///
/// Wire form: `scheme://kind/id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubjectRef {
    pub scheme: String,
    pub kind: String,
    pub id: String,
}

impl SubjectRef {
    /// Construct after validating each part is non-empty and follows
    /// scheme/kind/id syntax rules.
    pub fn new(
        scheme: impl Into<String>,
        kind: impl Into<String>,
        id: impl Into<String>,
    ) -> Result<Self, SubjectRefError> { /* ... */ }

    /// Parse the canonical wire form `scheme://kind/id`.
    pub fn parse(uri: &str) -> Result<Self, SubjectRefError> { /* ... */ }

    /// Render as the canonical wire form.
    pub fn to_uri(&self) -> String {
        format!("{}://{}/{}", self.scheme, self.kind, self.id)
    }
}

impl fmt::Display for SubjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_uri())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubjectRefError {
    #[error("scheme is empty")]
    EmptyScheme,
    #[error("kind is empty")]
    EmptyKind,
    #[error("id is empty")]
    EmptyId,
    #[error("scheme contains invalid character {0:?}; allow [a-z0-9-]")]
    InvalidSchemeChar(char),
    #[error("kind contains invalid character {0:?}; allow [a-z0-9-]")]
    InvalidKindChar(char),
    #[error("could not parse subject ref from {0:?}")]
    Malformed(String),
}
```

### Validation rules (proposed)

- `scheme` matches `[a-z][a-z0-9-]*` — the owning app id (e.g., `atlas`, `quorum`, `warden`, `movement`)
- `kind` matches `[a-z][a-z0-9-]*` — the domain noun (e.g., `acquisition-assets`, `unresolved-questions`, `dd-gates`)
- `id` is non-empty and contains no `/` or whitespace; otherwise opaque to Converge
- All three parts are case-sensitive on the wire but normalized to lowercase on construction

### Why three parts, not one

A single-newtype `SubjectUri(String)` would catch typos at the type level but lose structural decomposition. The cross-app resolver wants to dispatch on `scheme`. Helms wants to filter by `kind`. Apps want to extract `id` for storage keys. Splitting into three named parts at construction time costs ~30 lines of validation and avoids parsing strings at every consumption site.

A full RFC-3986 URI is overkill — we don't need fragments, query parameters, userinfo, or port. Schemes here are app-id labels, not protocols. Keeping a narrow grammar lets the validation be exhaustive.

## Where it appears

Integration surface, in proposed order of adoption:

| Surface | Today | After SubjectRef |
|---|---|---|
| Fact tagging | `ProposedFact` has no subject field | Add optional `subject: Option<SubjectRef>` to `ProposedFact` and `ContextFact` |
| Engagement state queries | Apps reconstruct subjects from payloads | `ContextState::facts_for_subject(subject)` returns all facts tagged with that subject |
| Cross-app citation resolver | Resolver takes `&str` if it exists | `trait CitationResolver { fn resolve(subject: &SubjectRef) -> Result<ResolvedCitation> }` |
| Helms readiness | Subjects are URI strings in JSON | Typed query: `helms::readiness_for(&subject)` |
| `IntentPacket.context` | Apps stuff URI strings as JSON | Apps may pass `SubjectRef` (serializes to the canonical wire form) |
| Replay traces | Subjects appear only in payload text | `ReplayTrace` may include `subject: Option<SubjectRef>` for index-on-replay |

Adoption is **strictly additive**. The wire forms apps use today (the literal strings `"atlas://acquisition-assets/shared-identity-core"`) round-trip through `SubjectRef::parse` and `SubjectRef::to_uri` byte-identically. Existing JSON payloads stay valid. Apps migrate at their own pace.

## Migration path

Five phases, each shippable independently:

1. **Land the type.** New module, public exports, parse/format round-trip tests, basic property tests on validation rules. No callers yet. (Single PR, ~150 lines + tests.)
2. **Optional field on `ProposedFact` and `ContextFact`.** Default `None` — backward compatible. Add a builder. (Single PR.)
3. **Citation resolver trait.** Define `CitationResolver` in Converge with no built-in implementations. Application kernels register their own. (Single PR, paired with an `application-kernel` PR registering Atlas's `atlas://` resolver.)
4. **Helms readiness federation.** Helms accepts `SubjectRef` in its readiness queries. (Helms PR.)
5. **Encourage app migration.** Apps that build URI strings today gradually swap to `SubjectRef`. The wire form is unchanged, so no breaking change is needed — this is purely an internal type-safety improvement.

## Open questions

These need maintainer input before code lands:

1. **Scheme registration.** Do we maintain a registry of valid schemes (`atlas`, `quorum`, `warden`, `movement`, etc.) or accept any scheme matching the regex? Open scheme is simpler; registered scheme catches squatting and typos. Recommendation: **accept any matching regex for now; registration becomes meaningful when there's a single platform-wide list of deployed apps**.
2. **Cross-app resolver location.** Does `CitationResolver` live in Converge, in `application-kernel`, or as a new `cross-app-citations` crate? Recommendation: trait in Converge, registration mechanism in `application-kernel`, app-specific resolvers in each app.
3. **Relationship to existing `Actor`.** `Actor` is who; `SubjectRef` is what. They're orthogonal — a fact has both. Do we want a combined type for "fact about subject S by actor A"? Recommendation: **no — keep them orthogonal**. Combinations belong on `ProposedFact` as separate fields.
4. **Should `kind` be typed (enum) instead of string?** No — kinds are app-domain words and app-extensible. Each app owns its kinds; Converge has no business enumerating them.
5. **Equality / hashing semantics.** Should `SubjectRef` equality be case-sensitive? Recommendation: normalize on construction (lowercase scheme and kind, preserve case in id), then byte-equal.
6. **Versioning.** If a subject's underlying record evolves, does `SubjectRef` change? Recommendation: **no — `SubjectRef` is identity, not snapshot**. Version goes on the fact, not the subject.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep using `&str` everywhere | No compile-time check, no schema, fragile under refactor. The audit caught this as the smell driving the proposal. |
| Single newtype `SubjectUri(String)` | Better than `&str` but loses scheme/kind/id structure that the resolver and Helms need. Forces re-parsing at every consumption site. |
| Full RFC-3986 URI type (e.g., the `url` crate) | Overkill. We don't use fragments, query parameters, userinfo, ports, or paths beyond a single id segment. Grammar can be exhaustive in ~30 lines. |
| Add `subject_scheme`, `subject_kind`, `subject_id` as three flat fields on `ProposedFact` | Splits one concept across three fields. Apps would invent their own grouping helper. Three-part struct is the right grouping. |
| Use `Actor` for both who and what | Conflates orthogonal concerns. `Actor` answers admission and auth; `SubjectRef` answers indexing and citation. |

## Sequencing

This proposal first lands as a KB doc for maintainer review. Once approved:

1. PR 1 — define the type (read-only, no integration).
2. PR 2 — add optional `subject` field to `ProposedFact` / `ContextFact`.
3. PR 3 — `CitationResolver` trait + `application-kernel` registration mechanism.
4. PR 4 — Helms readiness federation accepts `SubjectRef`.
5. App migrations as desired (Atlas first, then Quorum/Warden/Movement as their integration depth grows).

PRs 1–2 are uncontroversial mechanical work. PRs 3–4 need their own design alignment because they touch the cross-app boundary. Splitting the sequence lets PR 1 land quickly while the more contentious pieces get their own review.

## What this proposal does not do

- Doesn't propose a registry of deployed apps (separate, larger conversation).
- Doesn't propose a routing layer (that's `application-kernel`'s job).
- Doesn't enumerate valid `kind` values per app (app-domain, not Converge's concern).
- Doesn't change wire format of existing fact payloads or `IntentPacket.context` JSON (strict additive).

## See also

- `marquee-apps/atlas-integration/kb/Architecture/Upstream Types Audit.md` — full audit motivating this proposal.
- `marquee-apps/atlas-integration/crates/atlas-app/src/intent.rs` — current consumer of URI-as-string pattern.
- `crates/core/src/types/provenance.rs` — `Actor` (the orthogonal "who" type).
- `crates/pack/src/context.rs` — `ContextKey` (the closed-enum engagement state index).
