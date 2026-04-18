---
name: ADR-006 Typed-ID Newtype
description: Wrap string IDs in FactId newtype to enforce validation at compile time
type: adr
status: proposed
source: mixed
---

# ADR-006: Typed-ID Newtype Design

**Status**: Proposed (2026-04-18)  
**Author**: Claude / Engineering Team  
**Date**: 2026-04-18  
**Related**: EXP-002, ADR-005

## Context

Context IDs are currently unvalidated strings. The `Context::add_input()` and `Context::add_proposal()` APIs accept any `impl Into<String>`, which means:

- Empty IDs are accepted (no minimum length)
- IDs with null bytes, newlines, and other special characters are accepted (injection vulnerability)
- IDs can be unbounded in length (no max-length enforcement)
- Uppercase and special characters are accepted (inconsistent with spec)
- Validation errors occur at runtime (in the engine), not at the API boundary

### Evidence (EXP-002)

Property-based tests discovered six validation gaps:

| Gap | Impact | Current | Should Be |
|-----|--------|---------|-----------|
| Empty IDs | Confusing in logs | `ctx.add_input(key, "", "content")` → Ok(true) | Rejected |
| Null bytes | Injection vulnerability | `ctx.add_input(key, "id\0evil", "content")` → Ok(true) | Rejected |
| Newlines | Escaping bypass | `ctx.add_input(key, "id\ninjection", "content")` → Ok(true) | Rejected |
| Unbounded length | DoS via memory | 10,000-char IDs accepted | Max 128 chars |
| Whitespace-only | Ambiguous | `ctx.add_input(key, "   ", "content")` → Ok(true) | Rejected |
| Uppercase | Inconsistent | `ctx.add_input(key, "MyID", "content")` → Ok(true) | Lowercase only |

## Decision

Wrap string IDs in a `FactId` newtype that validates at construction time. This moves validation from runtime (engine) to compile-time (type system), making invalid IDs impossible to construct.

### The FactId Type

**Location**: `converge-pack::FactId` (public contract crate)

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactId(String);

impl FactId {
    /// Parse and validate an ID string.
    /// Valid format: `[a-z][a-z0-9:_-]{0,127}` (lowercase letter start, alphanumeric/colon/underscore/dash, max 128 chars)
    pub fn new(id: impl Into<String>) -> Result<Self, IdError> {
        let id_str = id.into();
        
        // Validate
        if id_str.is_empty() {
            return Err(IdError::Empty);
        }
        if id_str.len() > 128 {
            return Err(IdError::TooLong(id_str.len()));
        }
        if !id_str.chars().next().unwrap().is_ascii_lowercase() {
            return Err(IdError::InvalidStart);
        }
        if id_str.contains('\0') {
            return Err(IdError::ContainsNullByte);
        }
        if !id_str[1..].chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '_' || c == '-'
        }) {
            return Err(IdError::InvalidCharacters);
        }
        
        Ok(FactId(id_str))
    }
    
    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdError {
    Empty,
    TooLong(usize),
    InvalidStart,
    ContainsNullByte,
    InvalidCharacters,
}
```

### API Changes

#### Before
```rust
impl Context {
    pub fn add_input(&mut self, key: ContextKey, id: impl Into<String>, content: &str) -> Result<bool> { ... }
    pub fn add_proposal(&mut self, proposal: ProposedFact) -> Result<bool> { ... }
}

impl ProposedFact {
    pub fn new(key: ContextKey, id: impl Into<String>, content: &str, provenance: &str) -> Self { ... }
}
```

#### After
```rust
impl Context {
    pub fn add_input(&mut self, key: ContextKey, id: FactId, content: &str) -> Result<bool> { ... }
    pub fn add_proposal(&mut self, proposal: ProposedFact) -> Result<bool> { ... }
}

impl ProposedFact {
    pub fn new(key: ContextKey, id: FactId, content: &str, provenance: &str) -> Self { ... }
}
```

### Migration Path

**Phase 1** (future): Add `FactId` newtype to `converge-pack`, leave `Context` API as-is (dual APIs: `add_input()` takes string, `add_input_typed()` takes `FactId`)

**Phase 2** (future): Deprecate string-based API; recommend `FactId` in docs

**Phase 3** (future): Remove string-based API; break change on internal crates

**Phase 4** (future): Migrate `converge-core` to use `FactId` internally

## Consequences

### Positive
- ✅ **Type safety**: Invalid IDs are impossible to construct
- ✅ **Better errors**: `IdError::TooLong(256)` is more useful than runtime rejection
- ✅ **Documentation**: Type signature makes constraints explicit
- ✅ **Performance**: Validation happens once at boundary, not per-cycle in engine
- ✅ **Consistency**: All IDs follow the same format

### Negative
- ⚠️ **API breaking change**: Callers must construct `FactId` instead of passing strings
- ⚠️ **Verbosity**: `FactId::new("my-id")?` instead of just `"my-id"`
- ⚠️ **Migration cost**: Internal crates and examples need updates

### Mitigation
- Provide helper macro: `fact_id!("my-id")` for compile-time validation (stretch goal)
- Document migration path clearly in release notes
- Phase the change over 2-3 releases to allow downstream adoption

## Validation

**Evidence source**: EXP-002 property tests (`crates/core/tests/context_properties.rs`)

**Pre-implementation test status**:
- All 9 property tests currently pass (documenting gaps)
- After implementation: expect 6 negative tests to flip (empty, null byte, newline, unbounded, whitespace, uppercase now rejected)

**Post-implementation verification**:
- Rerun property tests; 6 that document gaps should now fail (as intended)
- Update tests to expect rejection for invalid IDs
- Verify no regression in valid ID roundtrip tests

## Open Questions

1. **Should we provide a `fact_id!()` macro for known-good literals at compile time?**
   - Reduces verbosity for common cases
   - Scope: defer to v3.5

2. **Should FactId support lossy normalization (auto-lowercase)?**
   - Trade-off: convenience vs explicitness
   - Decision: reject (fail-fast), require caller to normalize
   - Rationale: errors are better than silent changes

3. **Should we validate async (`pub async fn new()`)?**
   - Not needed; validation is pure logic (no I/O)
   - Keeper: sync API only

## Timeline

- **This sprint**: ADR review & community feedback
- **v3.5 (2026-07-15)**: Implement `FactId` newtype in `converge-pack`; add dual APIs
- **v3.6 (2026-08-15)**: Deprecate string APIs; migrate internals to `FactId`
- **v3.7+ (2026-09-15+)**: Remove deprecated APIs; `FactId` becomes mandatory

## References

- EXP-002: Context ID Validation Gaps (evidence collected via property tests)
- converge-pack: Public contract crate (owns types for downstream consumers)
- ADR-005: Type Ownership Boundaries

## See Also

- `kb/Experiments/EXP-002.md` — Evidence of validation gaps
- `crates/core/tests/context_properties.rs` — Property tests (6 gap tests)
