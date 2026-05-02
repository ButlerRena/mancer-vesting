# Mancer Vesting — Working Product Build Plan

## Overview
Upgrade the Week 3 scaffold to a fully working Solana token-distribution protocol with Merkle compression, root rotation, and comprehensive tests.

## Build Phases

### Phase 1: Data & State Updates (no logic yet)
**Goal:** `cargo check` passes with all new structs/errors/events
- [ ] Add `SameRoot` error variant to `errors.rs`
- [ ] Add `RootUpdated` event to `events.rs`
- [ ] Add `update_root.rs` instruction file (Accounts + stub handler)
- [ ] Add `solana-program = "2.1"` back to Cargo.toml (needed for keccak)
- [ ] Update `instructions/mod.rs` to include `update_root`
- [ ] Update `lib.rs` with `update_root` entry point
- [ ] **Verify:** `anchor build` passes

### Phase 2: Math Primitives
**Goal:** `leaf_hash`, `verify_merkle_proof`, `vested`, `get_vested_amount` fully implemented
- [ ] Implement `math/merkle.rs` — keccak256 leaf/node hash, proof verification
- [ ] Implement `math/schedule.rs` — cliff/linear/milestone vesting + cancel clamp
- [ ] Add Rust unit tests for `vested()` (4 branches + edge cases)
- [ ] **Verify:** `cargo test` unit tests pass

### Phase 3: Instruction Logic (in dependency order)
**Goal:** All 10 handlers have real bodies

#### 3a: Create + Fund (write path)
- [ ] `create_campaign` — validate inputs, init VestingTree, emit event
- [ ] `fund_campaign` — validate amount, check overflow, CPI transfer, emit event
- [ ] **Verify:** `anchor build` + `anchor test`

#### 3b: State Toggles
- [ ] `cancel_campaign` — set cancelled_at, emit event
- [ ] `pause_campaign` — set paused=true, emit event
- [ ] `unpause_campaign` — set paused=false, emit event
- [ ] **Verify:** `anchor build`

#### 3c: Claim (hot path — most complex)
- [ ] `claim` — pause check, beneficiary verify, schedule sanity, merkle proof, milestone guard, cancel clamp, state update, CPI transfer, emit
- [ ] **Verify:** `anchor build`

#### 3d: Root Rotation
- [ ] `update_root` — validate new root, update tree, emit event
- [ ] **Verify:** `anchor build`

#### 3e: Cleanup
- [ ] `withdraw_unvested` — grace period check, sweep vault, emit event
- [ ] `close_claim_record` — close conditions, close account, emit event
- [ ] `get_vested_amount` — delegate to math::schedule
- [ ] **Verify:** `anchor build` + `anchor test`

### Phase 4: TypeScript Merkle Tooling
**Goal:** `clients/ts/` produces byte-equal hashes to Rust
- [ ] Create `clients/ts/leaf.ts` — VestingLeaf encoder + leafHash
- [ ] Create `clients/ts/merkle.ts` — VestingMerkleTree class
- [ ] Create `clients/ts/index.ts` — re-exports
- [ ] **Verify:** `tsc --noEmit` passes

### Phase 5: Integration Tests
**Goal:** All scenarios from the brief pass
- [ ] Create `tests/utils/setup.ts` — provider, mint, ATA helpers
- [ ] Create `tests/utils/time.ts` — past/future timestamp helpers
- [ ] Create `tests/golden_vector.spec.ts` — byte-equal TS vs Rust hash gate
- [ ] Create `tests/vesting.spec.ts` — all scenarios:
  - Happy path: linear claim mid-stream
  - Invalid proof rejected
  - Pause blocks claim
  - Cancel mid-stream + clamp
  - Root rotation (kick recipient, others keep claiming)
  - Double-claim returns NothingToClaim
  - Withdraw unvested grace gate
- [ ] **Verify:** `anchor test` all green

### Phase 6: CI + README + Deploy
- [ ] Fix CI workflow (SBF toolchain issue)
- [ ] Update README for working product
- [ ] Redeploy to devnet
- [ ] **Verify:** CI green, devnet program updated

## Risk Notes
- `@coral-xyz/anchor ^1.0.0` doesn't exist on npm — will use `@anchor-lang/core ^1.0.0`
- The brief's `Vesting111...` placeholder is invalid — keep our real keypair
- CI `cargo-build-sbf` panic needs the SBF toolchain pre-fetched
- Context window is at ~65% — may need compaction or sub-agents for Phase 5

## Time Estimate
| Phase | Estimated Time |
|---|---|
| Phase 1: Data updates | 5 min |
| Phase 2: Math | 10 min |
| Phase 3: Instructions | 20 min |
| Phase 4: TS tooling | 10 min |
| Phase 5: Integration tests | 25 min |
| Phase 6: CI + deploy | 10 min |
| **Total** | **~80 min** |
