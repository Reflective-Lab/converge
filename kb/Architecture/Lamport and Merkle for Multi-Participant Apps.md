# Lamport and Merkle for Multi-Participant Apps

Authoritative spec for marquee-apps with concurrent multi-participant state. Standardizes adoption of `converge_core::integrity::{LamportClock, ContentHash, MerkleRoot, IntegrityProof}` so apps stop inventing their own ordering and integrity primitives.

## Named adopters

| App | Adoption depth | Status |
|---|---|---|
| Quorum (`marquee-apps/quorum-sense`) | Full: Inquiry, Signal, Hypothesis, Probe, Round, BreakoutBranch, Synthesis | Live multi-participant survey product — primitives mandatory |
| Tally (`marquee-apps/tally-escrow`) | Full: Engagement, Condition, StateTransition, Commitment, Release | Bilateral escrow — finality requires matching roots on both sides |
| Atlas (`marquee-apps/atlas-integration`) | Partial today (`lamport_at` on `UnresolvedAcquisitionQuestion`); will gain `signal_history_root` and an event chain on `IntegrationCandidate` | Acquisition evidence spike was the first adopter at smaller scope |

Other marquee-apps with multi-actor / state-evolving semantics (Scout, Plumb, Fathom, Vouch) inherit the same spec when they reach that depth.

## Locked decisions

1. **Lamport scope: per-engagement.** One clock per inquiry / engagement / candidate. All writes go through the same clock cell (interior mutability + lock). Per-participant vector clocks deferred until multi-writer race conditions are observed in practice; the wire shape stays compatible with that future migration.
2. **Merkle granularity: linear chain.** Append-only `root_new = ContentHash::combine(root_old, event_hash)`. No tree, no rebalancing, no succinct membership proofs. Verifiers re-fold the whole event sequence. Tree-based migration deferred until external auditors who need compact proofs arrive.
3. **Branch merge: stay forked.** A branch (Quorum BreakoutBranch; Tally counter-proposal track) has its own clock and its own chain. The parent engagement commits to the branch's `id` + its `root_at_fork`, never to the branch's evolving contents. Reconciliation happens through an explicit **Synthesis event** in the parent that references the branch's terminal root — the merge is *content-addressed reference*, not a chain operation.
4. **Shared spec, app-specific addenda.** This doc is the canonical wire/type vocabulary. Each app adds an addendum in its own KB describing the per-type canonical-bytes serialization and the read endpoints.

## Vocabulary

All marquee-apps use these types directly from `converge_core::integrity`:

- `LamportClock` — per-engagement logical clock; `.tick()`, `.update(received)`, `.time()`
- `ContentHash` — SHA-256 over canonical bytes; `.compute(...)`, `.combine(left, right)`, `.to_hex()`
- `MerkleRoot(pub ContentHash)` — chain head; `.0` is the accumulated hash
- `IntegrityProof` — bundles `lamport_at`, `event_history_root`, `event_count` for transmission to external observers

No app may define its own newtype or alias for these. App-side fields use the upstream types directly:

```rust
use converge_core::integrity::{LamportClock, MerkleRoot};

pub struct Inquiry {
    // ...
    pub lamport_at: LamportClock,
    pub event_history_root: Option<MerkleRoot>,
}
```

For wire serialization, `LamportClock` serializes as its inner `u64` (already implemented via `#[derive(Serialize, Deserialize)]` on the type); `MerkleRoot` serializes as a hex string (32-byte ContentHash → 64 hex chars). Atlas's current `lamport_at: u64` field in `UnresolvedAcquisitionQuestion` is the wire form of `LamportClock` — the type migration is mechanical when Quorum's domain crate gains a `converge-core` dependency.

## Wire shape

### Per-event

Every event type (Signal, Hypothesis, Probe, Condition, StateTransition, …) carries:

| Field | Type | Semantics |
|---|---|---|
| `id` | `Uuid` | event identity |
| `lamport_at` | `LamportClock` (wire: `u64`) | logical time at admission |
| `content_hash` | `ContentHash` (wire: hex `String`) | SHA-256 of the canonical-bytes representation |
| `parent_root` | `Option<MerkleRoot>` (wire: hex `String`) | the chain head this event is appended onto; `None` only for the genesis event |

Optional but recommended:

| Field | Type | Semantics |
|---|---|---|
| `participant_id` | `Option<ParticipantId>` | who submitted; sets up the migration to per-participant clocks without a wire-shape break |
| `received_lamport` | `Option<u64>` | the participant's local clock at submission; `apply_signal` uses `max(local, received) + 1` |

### Per-aggregate

Every aggregate type (Inquiry, Engagement, IntegrationCandidate, …) carries:

| Field | Type | Semantics |
|---|---|---|
| `lamport_at` | `LamportClock` (wire: `u64`) | current engagement clock |
| `event_history_root` | `Option<MerkleRoot>` (wire: hex `String`) | head of the linear chain over all admitted events |
| `event_count` | `u64` | number of admitted events (for O(1) verification scaffolding) |

## Canonical-bytes rule (for hashing)

`event_hash = ContentHash::compute(canonical_bytes_of(event))`.

Canonical bytes for an event are:

1. The event struct serialized via `serde_json::to_vec` after sorting all map keys lexicographically (use `serde_json::to_value` then a stable-stringify pass), with the `content_hash` and `parent_root` fields *excluded* (those describe the chain, not the event content).
2. Newline-separated. Apps MAY use a stricter canonical form (e.g., CBOR with deterministic encoding) as long as the choice is documented in the app's addendum.

**Why exclude content_hash / parent_root from the hash input:** they're chain metadata, not content. Including them would make the hash self-referential.

## Lamport semantics

### Tick (local write)

```rust
fn admit_local_event(engagement: &mut Engagement, event: &mut Event) {
    engagement.clock.tick();
    event.lamport_at = engagement.clock;
}
```

### Update (cross-process write)

```rust
fn admit_remote_event(engagement: &mut Engagement, event: &mut Event, participant_lamport: u64) {
    engagement.clock.update(participant_lamport);  // sets to max(local, received) + 1
    event.lamport_at = engagement.clock;
}
```

### Persistence

The clock's current value is recoverable from storage: on engagement load, scan the event log for that engagement and set `clock = LamportClock::at(max(events.iter().map(|e| e.lamport_at)) + 1)`. The clock field on the aggregate is a denormalized snapshot for fast reads; the event log is the source of truth.

### Concurrent admit serialization

When multiple admits race for the same engagement, the clock must be advanced atomically. Implementations use either:

- `Arc<Mutex<LamportClock>>` on the engagement handle (simple, correct, contention bounded by per-engagement traffic)
- Per-engagement tokio mutex around the admission critical section (same shape, async-friendly)

Per-engagement contention is acceptable because the bottleneck is human submission rate, not machine throughput.

## Merkle linear chain

### Construction

After admission of event `e_n`:

```rust
fn append_to_chain(engagement: &mut Engagement, event_hash: ContentHash) {
    let new_head = match engagement.event_history_root {
        None => MerkleRoot(event_hash),                          // genesis
        Some(prior) => MerkleRoot(ContentHash::combine(&prior.0, &event_hash)),
    };
    engagement.event_history_root = Some(new_head);
    engagement.event_count += 1;
}
```

### Verification

External observers (Atlas reading Quorum; auditors reading Tally) verify by re-deriving:

```rust
fn rederive_root(events: &[Event]) -> Option<MerkleRoot> {
    events.iter().fold(None, |acc, event| {
        let event_hash = ContentHash::compute(&canonical_bytes(event));
        Some(match acc {
            None => MerkleRoot(event_hash),
            Some(prior) => MerkleRoot(ContentHash::combine(&prior.0, &event_hash)),
        })
    })
}

assert_eq!(rederive_root(&fetched_events), claimed_root);
```

Mismatch → tampering, silent edit, or non-deterministic canonical-bytes implementation (the last being a spec bug, not an attack).

### Storage

Events go to `runway_storage::EventLog` (already in use — atlas-server's room_memory.rs pattern). The aggregate's denormalized `lamport_at` and `event_history_root` fields go to `DocumentStore` for fast reads. The event log is authoritative; the document snapshot is a cache.

Atomicity: implementations SHOULD write the event to the log *before* updating the aggregate snapshot, so a crash between the two leaves the system recoverable (the log replay rebuilds the snapshot). This mirrors the discipline Atlas's room-memory implementation already follows.

## Branch semantics (option 3 — branches stay forked)

A branch (Quorum BreakoutBranch; Tally counter-proposal track) is a forked sub-engagement with its own state:

```rust
pub struct Branch {
    pub id: BranchId,
    pub parent_engagement_id: EngagementId,
    pub root_at_fork: MerkleRoot,           // parent's root at the moment of fork
    pub lamport_at_fork: LamportClock,      // parent's clock at the moment of fork
    pub clock: LamportClock,                // branch's own clock, initialized from lamport_at_fork
    pub event_history_root: Option<MerkleRoot>,  // branch's own chain head
    pub event_count: u64,
}
```

The parent engagement commits to the *existence* of the branch (its `id` and `root_at_fork`) but does **not** absorb branch events into its own chain. Branch events are admitted only into the branch's chain.

### Reconciliation via Synthesis event

When a branch is reconciled back, a `Synthesis` event in the parent references the branch's terminal root:

```rust
pub struct Synthesis {
    pub branch_id: BranchId,
    pub branch_terminal_root: MerkleRoot,    // the branch's event_history_root at synthesis time
    pub branch_terminal_lamport: LamportClock,
    pub summary: String,
    pub agreements: Vec<String>,
    pub unresolved_dissent: Vec<String>,
    pub evidence_gaps: Vec<String>,
}
```

The Synthesis event itself goes through the parent's chain (incrementing parent clock, parent root). Its `content_hash` includes the cited branch_terminal_root, so the parent's chain commits to "we synthesized from a branch whose terminal state was *exactly* this root." Verifiers can fetch the branch's events and confirm the cited root matches.

**Properties this gives:**

- Branch contents stay in branch storage; parent chain doesn't bloat
- Synthesis is a content-addressed commit, not a state merge — no merge conflicts at the chain level
- Multiple branches can be synthesized into one parent event by referencing multiple terminal roots in one Synthesis
- A branch that never reconciles never appears in parent's chain beyond the fork commitment — the parent's chain stays focused on what reached synthesis

## Integrity verification API

Every multi-participant app exposes two read endpoints per engagement:

```
GET /{engagement_kind}/{id}/integrity → IntegrityProof
GET /{engagement_kind}/{id}/events    → ordered Vec<Event>
```

Where `IntegrityProof`:

```rust
pub struct IntegrityProof {
    pub lamport_at: LamportClock,
    pub event_history_root: Option<MerkleRoot>,
    pub event_count: u64,
    pub branches: Vec<BranchHead>,      // committed-but-not-synthesized branch heads
}

pub struct BranchHead {
    pub branch_id: BranchId,
    pub root_at_fork: MerkleRoot,
    pub current_root: Option<MerkleRoot>,
    pub current_lamport: LamportClock,
}
```

Atlas, Helms, and external auditors verify by:

1. `GET /{engagement}/{id}/integrity` → claimed_proof
2. `GET /{engagement}/{id}/events` → events_seq
3. Recompute `rederive_root(&events_seq)` → derived_root
4. `assert_eq!(claimed_proof.event_history_root, derived_root)`

For Atlas's cross-app citation: when Atlas pulls a Quorum question via `quorum://unresolved-questions/{id}`, it gets the question's `lamport_at` and (once the Atlas acquisition flow gains the field) `event_history_root`. Atlas records both in its evidence packet. Future readers of Atlas's evidence can cross-check by hitting Quorum's verification endpoints.

## Per-app addenda

Each app writes its own `kb/Architecture/Integrity Adoption.md` documenting:

- Which types are events vs aggregates
- The canonical-bytes implementation for each event type
- The per-engagement clock placement (which struct holds the `Arc<Mutex<LamportClock>>`)
- Read endpoint paths for `/integrity` and `/events`
- Storage layout for the event log + aggregate snapshot
- Branch semantics specific to the app's domain

The addendum citation chain back to this spec is mandatory: each addendum's frontmatter includes `spec: stack/bedrock-platform/converge/kb/Architecture/Lamport and Merkle for Multi-Participant Apps.md`.

## Implementation sequencing

For Quorum (the most advanced adopter):

1. **Add `converge-core` dependency** to `quorum-domain` and `quorum-app`. Replace `lamport_at: u64` with `lamport_at: LamportClock` on existing types (`UnresolvedAcquisitionQuestion`). Wire shape unchanged.
2. **Per-inquiry clock + chain** on `InquiryThread`. Hold `Arc<Mutex<LamportClock>>` on the engagement; tick on every event admission. Add `event_history_root: Option<MerkleRoot>` field.
3. **Signal event** carries `lamport_at`, `content_hash`, `parent_root` (its inputs from the prior aggregate state). Canonical bytes = signal content + role + provenance (excluding the chain fields).
4. **Hypothesis, Probe** likewise; the `content_hash` of each includes references to the source events it derives from.
5. **Round** as a sub-aggregate: rounds have their own scoped chain head; round close emits a "round terminal" event into the parent inquiry chain referencing the round's root.
6. **BreakoutBranch** with `root_at_fork`, own clock, own chain. Branch creation = parent event "branch opened (branch_id, root_at_fork)"; branch lives in its own storage.
7. **Synthesis** event in parent inquiry references branch terminal roots.
8. **Verification endpoints**: `GET /inquiry/{id}/integrity` and `GET /inquiry/{id}/events`.
9. **Atlas acquisition question flow**: gains `signal_history_root` field. The Atlas mirror struct gains it too. Smoke phase 3 also verifies the chain re-derives.

For Tally (design-only today): the same primitives apply on Engagement / Condition / StateTransition / Commitment / Release. Tally's bilateral nature adds one wrinkle: both parties must observe matching roots at finality — there are two engagements (one per party's view), and `Release` requires both engagement roots to agree on the state being released.

For Atlas: post-Spike 1, `IntegrationCandidate` events (proposed → challenged → promoted/blocked/rejected) form a chain. This makes Atlas's evidence packet self-verifying for the candidate's history.

## Conformance checklist

- [ ] App's `quorum-domain` / `tally-domain` / `atlas-domain` depends on `converge-core` for `LamportClock`, `ContentHash`, `MerkleRoot`
- [ ] Event types carry `lamport_at`, `content_hash`, `parent_root` (and optionally `participant_id`, `received_lamport`)
- [ ] Aggregate types carry `lamport_at`, `event_history_root`, `event_count`
- [ ] Per-engagement clock is held under `Arc<Mutex<LamportClock>>` or tokio equivalent
- [ ] Canonical-bytes function for each event type documented in app addendum
- [ ] `/integrity` and `/events` endpoints exposed per engagement
- [ ] Branch creation commits `(branch_id, root_at_fork)` to parent chain; branch contents stay in branch storage
- [ ] Synthesis events reference branch terminal roots, not branch contents
- [ ] App's CHANGELOG entry cites this spec

## Open items (deferred)

- **Per-participant vector clocks.** When concurrent multi-writer scenarios produce ordering disputes that per-inquiry Lamport can't disambiguate. Wire shape supports the migration via the optional `participant_id` and `received_lamport` fields on events.
- **Tree-based Merkle.** When external auditors need compact membership proofs without downloading the full event log. Migration: replace `event_history_root: MerkleRoot` with a tree node; existing chain-based verifiers continue to work against a flattened representation.
- **Cross-app chain composition.** When Atlas cites Quorum and wants its own evidence chain to *commit* to the Quorum root at citation time (not just *record* it). Likely a `CrossAppCitation` event type that carries `(cited_uri, cited_root, cited_lamport_at)`.
- **Replay determinism.** The canonical-bytes function must be exactly reproducible across app versions for chain verification to survive software upgrades. Versioning the canonical-bytes function via a `canonical_bytes_version` field on events is the likely path; not yet specified.

## See also

- `crates/core/src/integrity.rs` — the primitive surface (`LamportClock`, `ContentHash`, `MerkleRoot`, `IntegrityProof`).
- `marquee-apps/quorum-sense/kb/Architecture/Merkle Signal Chain Followup.md` — the precursor follow-up doc, now superseded by this spec.
- `marquee-apps/atlas-integration/crates/atlas-server/src/room_memory.rs` — `EventLog` + `DocumentStore` split pattern that the storage layout adopts.
- `marquee-apps/quorum-sense/crates/quorum-server/src/main.rs` — current multi-participant routes (`/inquiry`, `/signal`, `/rounds/next`, `/breakouts`, `/breakouts/{branch_id}/synthesis`) that this spec applies to.
