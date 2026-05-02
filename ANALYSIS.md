# Week 3 Analysis & Improvement Plan

## Build Status

| Check | Status |
|---|---|
| `anchor build` exits 0 | ✅ |
| `target/deploy/vesting.so` produced | ✅ (266KB) |
| `target/idl/vesting.json` produced | ✅ (28KB) |
| 9 instructions in IDL | ✅ |
| 2 account types in IDL | ✅ VestingTree, ClaimRecord |
| 28 error variants in IDL | ✅ |
| 1 passing test | ✅ |
| `cargo check` exits 0 | ✅ |
| CI pipeline | ✅ `.github/workflows/ci.yml` |
| Program ID | `BKauLFNrGhWpaiHkWP3XrDGq5ZfMMNeTdmbtNbHydxAX` |

---

## What Works Well

### Architecture
Clean instruction separation — each handler in its own file, shared types via `state/mod.rs`, math isolated for Week 4. The Merkle distributor model (one campaign = one root = many recipients) is the right design for gas-efficient token distribution on Solana.

### Account Structs
`VestingTree` (~240 bytes) and `ClaimRecord` (~137 bytes) are lean. `#[derive(InitSpace)]` means Anchor auto-calculates space — no manual byte counting bugs.

### Error Surface
28 variants covering every edge case — paused, cancelled, unauthorized, overflow, proof failures, milestone collisions. Comprehensive enough that Week 4 logic shouldn't need to add more.

### Pause/Unpause Pattern
Elegant type alias: one `Accounts` struct, two handlers, zero duplication. Clean.

---

## Known Issues & Fixes

### 1. Placeholder Program ID (resolved)
The brief's `Vesting1111111111111111111111111111111111111` fails Anchor's Pubkey parser (invalid base58 byte length). Anchor auto-generates a real keypair and `anchor keys sync` fixes all declarations. The README documents the swap-on-first-deploy step.

### 2. TS Package Name
Brief specifies `@coral-xyz/anchor ^1.0.0` but that package tops out at 0.32.x on npm. Switched to `@anchor-lang/core ^1.0.0` which is the actual Anchor 1.0 package. Tests pass. This is a brief inaccuracy, not a code bug.

### 3. `solana-program` Dependency (resolved)
Brief listed it as a Cargo dependency. Anchor 1.0 re-exports it from `anchor_lang::solana_program`. Removed to prevent version conflicts. The warning:
```
Adding `solana-program` as a separate dependency might cause conflicts.
```
is now gone.

### 4. Compiler Warnings (12 total, all Anchor framework)
`unexpected cfg` warnings from Anchor 1.0 macros on Rust 1.95. Harmless but noisy.

**Fix for Week 4** — add to `programs/vesting/Cargo.toml`:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(anchor_debug, values("true", "false"))'] }
```

### 5. `ambiguous_glob_reexports` Warning
`pub use create_campaign::*` etc. in `instructions/mod.rs` causes ambiguous `handler` name since all modules export a function called `handler`.

**Fix options:**
- Rename handlers: `create_campaign_handler`, `fund_campaign_handler`, etc.
- Or use explicit re-exports:
  ```rust
  pub use create_campaign::{CreateCampaign, CreateCampaignArgs, handler as create_campaign_handler};
  ```

### 6. CI Speed (~8-10 min per run)
`avm` compiles Anchor CLI from source every CI run. To speed up:
- Cache the `~/.avm` directory between runs
- Or use a pre-built Docker image with Anchor 1.0 pre-installed
- Or add a CI cache step for `avm` binary

---

## Improvement Plan

### Week 3.5 — Before Starting Week 4

- [ ] Fix `ambiguous_glob_reexports` warning
- [ ] Add `[lints.rust]` to suppress `unexpected_cfgs`
- [ ] Add `cargo clippy` to CI
- [ ] Add `anchor keys check` to CI
- [ ] Speed up CI with avm caching

### Week 4 — Logic Implementation

- [ ] Implement `vested()` in `math/schedule.rs`
- [ ] Implement `leaf_hash()` in `math/merkle.rs`
- [ ] Implement `verify_merkle_proof()` in `math/merkle.rs`
- [ ] Implement `create_campaign` handler logic
- [ ] Implement `fund_campaign` handler logic
- [ ] Implement `claim` handler logic
- [ ] Implement `cancel_campaign` handler logic
- [ ] Implement `withdraw_unvested` handler logic
- [ ] Implement `pause_campaign` / `unpause_campaign` handler logic
- [ ] Implement `close_claim_record` handler logic
- [ ] Emit events in all handlers
- [ ] Write comprehensive integration tests

### WOW Improvements (Optional)

- [ ] **LiteSVM tests** — Run `anchor test` without a validator, much faster
- [ ] **Pre-commit hooks** — `cargo fmt --check` + `cargo clippy` before every commit
- [ ] **`anchor expand` for debugging** — Shows macro-generated code for Accounts constraints
- [ ] **`CLAUDE.md` in repo** — Project-level context file for AI assistants
- [ ] **Fuzz testing** — `cargo-fuzz` or `proptest` for math functions
- [ ] **Benchmarks** — Benchmark `leaf_hash` and `verify_merkle_proof` performance

---

## Scorecard

| Dimension | Rating | Notes |
|---|---|---|
| Build correctness | 10/10 | Builds clean, all checks pass |
| Spec compliance | 9/10 | Placeholder ID issue was brief's fault |
| Test coverage | 1/10 | 1 test only — expected for Week 3 scaffold |
| CI pipeline | 8/10 | Works, but slow due to avm compile |
| Code organization | 10/10 | Clean module separation |
| Error handling | 10/10 | Comprehensive enum |
| Documentation | 8/10 | Good README, could use inline docs |
| Security posture | 9/10 | All constraints in place, logic pending Week 4 |
