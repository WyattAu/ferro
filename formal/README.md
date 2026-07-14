# Ferro Formal Verification

This directory contains Lean4 proofs for core Ferro algorithms.

## Prerequisites

- [Lean4](https://lean-lang.org/) v4.12.0 (managed via elan)
- [Lake](https://github.com/leanprover/lake) (bundled with Lean4)

## Setup

```bash
# Install Lean4 via elan
curl https://elan.lean-lang.org/elan-init.sh -sSf | sh -s -- -y

# Navigate to formal directory
cd formal

# Build all proofs
lake build
```

## Project Structure

```
formal/
├── lakefile.lean              # Lake build configuration
├── lean-toolchain             # Lean4 version pinning
├── Ferro/
│   ├── Basic.lean             # Basic definitions and proofs
│   ├── DataTypes.lean         # Core data structure invariants
│   ├── CircuitBreaker.lean    # Circuit breaker state machine proofs
│   ├── Authentication.lean    # Authentication token properties
│   ├── RateLimiter.lean       # Token bucket rate limiter proofs
│   ├── PathValidation.lean    # Path traversal prevention proofs
│   ├── Cache.lean             # LRU cache eviction proofs
│   └── HashConsistency.lean   # Hash consistency proofs
└── README.md                  # This file
```

## Verified Components

### Circuit Breaker State Machine
- State transitions: Closed → Open → HalfOpen → Closed
- Threshold-based failure counting
- Recovery via successful requests

### Authentication Tokens
- Token validity via expiration timestamps
- User ID preservation across refreshes

### Data Structures
- Content-addressable hash validation (SHA-256 format)
- Structural equality properties

### Rate Limiter (Token Bucket)
- Initial state has full tokens (`init_has_full_tokens`)
- Refill never exceeds maxTokens (`refill_never_exceeds_max`)
- Consume fails when empty (`consume_fails_when_empty`)
- Consume decreases tokens (`consume_result_decreases`)
- Zero-cost consume is identity (`consume_zero_is_identity`)
- Overflow consumption empties bucket (`consume_overflow_empties`)

### Path Validation (Traversal Prevention)
- Root path is valid (`root_path_is_valid`)
- Single segment path is valid if not ".." (`single_segment_valid`)
- Traversal path is invalid (`traversal_path_invalid`)
- Empty path is valid (`empty_path_valid`)
- `noTraversal` implies `isValid` (`noTraversal_implies_isValid`)

### LRU Cache Eviction
- Empty cache has size 0 (`empty_size`)
- Lookup on empty cache returns none (`lookup_empty`)
- Insert on empty cache adds one entry (`insert_empty`)
- Clear empties entries (`clear_empties_entries`)
- Insert when not full increases size (`insert_not_full_increases`)
- Remove decreases size by at most 1 (`remove_decreases`)

### Hash Consistency
- Same input produces same hash (`hash_deterministic`)
- Hash equality is struct equality (`hash_struct_eq`)
- Collision resistance axiom (`hash_collision_resistant`)
- Hash is pure (`hash_pure`)
- BEq consistency (`hash_beq_reflect`)

## CI Integration

Formal proofs are verified on every push and PR via `.github/workflows/formal_verification.yml`.
