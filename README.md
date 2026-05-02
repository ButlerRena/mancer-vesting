# Mancer Vesting Protocol

A Solana token-distribution protocol using Merkle compression. One 32-byte
Merkle root on-chain stands in for an entire recipient list, so a campaign
of 10,000 recipients costs a project ~$0.42 instead of ~$1,990 on
per-recipient-PDA designs.

## Program Address

`BKauLFNrGhWpaiHkWP3XrDGq5ZfMMNeTdmbtNbHydxAX`

## Features

- **Merkle compression** — keccak256 leaf hashing, on-chain proof verification
- **Customizable vesting** — cliff, linear, and milestone schedules
- **Per-recipient clawback** — rotate the Merkle root to revoke individual allocations
- **Campaign-wide cancel** — with 7-day grace period before unvested funds can be withdrawn
- **Pause / unpause** — admin can freeze all claims
- **TS Merkle tooling** — `clients/ts/` with byte-equal Rust ↔ TS golden vector guarantee

## Instructions (10)

| Instruction | Description |
|---|---|
| `create_campaign` | Initialize a vesting campaign with schedule config |
| `fund_campaign` | Deposit SPL tokens into the campaign vault |
| `claim` | Recipient claims vested tokens (Merkle proof required) |
| `update_root` | Rotate Merkle root (clawback individual recipients) |
| `pause_campaign` | Admin pauses all claims |
| `unpause_campaign` | Admin resumes claims |
| `cancel_campaign` | Admin cancels the campaign (7-day grace before withdrawal) |
| `withdraw_unvested` | Admin withdraws unvested tokens after grace period |
| `close_claim_record` | Close a claim record PDA to reclaim rent |
| `get_vested_amount` | Read-only view of a recipient's vested amount |

## Error Handling (29 variants)

Full coverage including: `InvalidMerkleProof`, `AlreadyClaimed`, `NothingToClaim`,
`CampaignPaused`, `CampaignCancelled`, `GracePeriodNotElapsed`, `SameRoot`,
`InsufficientFunds`, `Unauthorized`, `IncorrectOwner`, and more.

## Test Results

```
anchor test
  ✔ encoded leaf is exactly 70 bytes
  ✔ leafHash is deterministic and 32 bytes
  ✔ matches the Rust golden hash (TS ↔ Rust byte-equal)
  ✔ happy path: linear claim mid-stream transfers proportional amount
  ✔ invalid proof is rejected
  ✔ pause blocks claim, unpause restores it
  ✔ cancel + claim returns pre-cancel vested only
  ✔ update_root: kicks a recipient — old proof fails, others keep claiming

  8 passing (20s)
```

### Golden Vector Guarantee

The TypeScript `VestingLeaf` encoder in `clients/ts/leaf.ts` produces
byte-identical output to the Rust `leaf_hash()` in `math/merkle.rs`.
This is verified at test time — any mismatch fails CI.

## Prerequisites

- **Rust** (stable):
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Solana CLI** ≥ 2.1:
  `sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.6/install)"`
- **Anchor CLI** 1.0.0 via `avm`:
  ```
  cargo install --git https://github.com/coral-xyz/anchor avm --force
  avm install 1.0.0
  avm use 1.0.0
  ```
- **Node** ≥ 22 + Yarn (or npm).

## Setup

```
git clone https://github.com/ButlerRena/mancer-vesting.git
cd mancer-vesting
yarn install
anchor build
```

## Test

```
anchor test
```

Runs 8 integration tests against a local validator.

## Deploy to devnet

```
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --provider.cluster devnet
```

## Project Layout

```
programs/vesting/src/
├── lib.rs                # program entry, #[program] mod
├── constants.rs          # protocol constants
├── errors.rs             # 29 VestingError variants
├── events.rs             # 8 event structs
├── state/
│   ├── vesting_tree.rs   # campaign account
│   ├── claim_record.rs   # per-recipient claim tracker
│   └── leaf.rs           # VestingLeaf struct
├── instructions/
│   ├── create_campaign.rs
│   ├── fund_campaign.rs
│   ├── claim.rs
│   ├── update_root.rs    # per-recipient clawback
│   ├── pause_campaign.rs
│   ├── cancel_campaign.rs
│   ├── withdraw_unvested.rs
│   ├── close_claim_record.rs
│   └── get_vested_amount.rs
└── math/
    ├── merkle.rs         # leaf_hash (keccak256), verify_merkle_proof
    └── schedule.rs       # vested (cliff/linear/milestone), get_vested_amount

clients/ts/
├── leaf.ts               # VestingLeaf encoder
├── merkle.ts             # tree builder + proof generator
└── index.ts              # re-exports

tests/
├── golden_vector.spec.ts # TS ↔ Rust byte-equality test
├── vesting.spec.ts       # integration tests (7 scenarios)
└── utils/
    ├── setup.ts          # provider & helpers
    └── time.ts           # time manipulation utilities
```

## Tech Stack

- **Solana** 2.1.6
- **Anchor** 1.0.0
- **Rust** 1.95.0
- **TypeScript** with `@anchor-lang/core ^1.0.0`
- **Node** 22

## License

MIT
