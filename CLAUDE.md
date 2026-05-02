# CLAUDE.md — Project Context

## Project
Mancer Vesting Protocol — Solana token-distribution using Merkle compression.
Team 7, Mancer Scholarship.

## Current State
Week 3 scaffold is complete and deployed to devnet. All instruction handlers are empty stubs (`Ok(())`). Now upgrading to working product with full logic.

## Program ID
`BKauLFNrGhWpaiHkWP3XrDGq5ZfMMNeTdmbtNbHydxAX`

## Build Commands
```bash
source "$HOME/.cargo/env"
export PATH="/root/.local/share/solana/install/active_release/bin:$PATH"
cd /root/.openclaw/workspace/mancer-vesting
anchor build        # build the program
anchor test         # run tests (needs local validator)
cargo test          # Rust unit tests only
```

## Key Decisions
- TS package: `@anchor-lang/core` (not `@coral-xyz/anchor` — 1.0 doesn't exist there)
- Program keypair is committed at `target/deploy/vesting-keypair.json`
- `solana-program` dependency IS needed (for keccak256 in merkle.rs)

## File Layout
See BUILD_PLAN.md for the phased build plan.
See BUILD_BRIEF.md for the full specification with all code.
