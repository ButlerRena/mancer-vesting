# Mancer Vesting Protocol

A Solana token-distribution protocol using Merkle compression. One 32-byte
Merkle root on-chain stands in for an entire recipient list, so a campaign
of 10,000 recipients costs a project ~$0.42 instead of ~$1,990 on
per-recipient-PDA designs.

## Program Address

`BKauLFNrGhWpaiHkWP3XrDGq5ZfMMNeTdmbtNbHydxAX`

### Features

- Merkle-compressed recipient lists (keccak256)
- Customizable vesting schedules (cliff, linear, milestone)
- Per-recipient clawback via Merkle root rotation
- Campaign-wide cancel with 7-day grace period
- Pause/unpause support
- TS Merkle tooling (`clients/ts/`)

## Prerequisites

- **Rust** (stable):
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Solana CLI** ≥ 2.1:
  `sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.0/install)"`
- **Anchor CLI** 1.0.0 via `avm`:
  ```
  cargo install --git https://github.com/coral-xyz/anchor avm --force
  avm install 1.0.0
  avm use 1.0.0
  ```
- **Node** ≥ 20 + Yarn (or npm).
- A Solana keypair: `solana-keygen new` if you don't already have
  `~/.config/solana/id.json`.

## Setup

```
git clone https://github.com/ButlerRena/mancer-vesting.git
cd mancer-vesting
yarn install        # or: npm install
```

## Build

```
anchor build
```

Produces `target/deploy/vesting.so` and `target/idl/vesting.json`.

## Test

```
anchor test
```

Runs the integration test suite against an in-process LiteSVM. One green
test in Week 3 (program loads).

## Deploy to devnet

```
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --provider.cluster devnet
```

After the first devnet deploy, copy the printed program ID into:
- `Anchor.toml` → `[programs.devnet] vesting = "..."`
- `programs/vesting/src/lib.rs` → `declare_id!("...")`

Then `anchor build` again so the IDL matches the on-chain ID.

## Project layout

```
programs/vesting/src/
├── lib.rs            # entry, #[program] mod
├── constants.rs
├── errors.rs         # VestingError variants
├── events.rs         # 8 event structs
├── state/            # VestingTree, ClaimRecord, VestingLeaf
├── instructions/     # 8 instruction modules
└── math/             # schedule + Merkle helpers (Week 4)
tests/                # ts integration tests
.github/workflows/    # CI (anchor build + anchor test)
```

## What's implemented

All 10 instructions with full logic, integration tests (8/8 passing),
and TS Merkle client tooling.
