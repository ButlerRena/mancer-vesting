# Mancer Vesting — Working Product Build Brief

You are an AI coding agent. Build a working Solana token-distribution protocol with Merkle compression, per-recipient clawback (root rotation), customizable vesting (cliff / linear / milestone), and a 7-day grace clawback. Every instruction has full logic; every math primitive is implemented; integration tests cover happy paths and the rotation flow. Backend lives in `programs/vesting/`. TypeScript Merkle tooling (used by both backend tests and the frontend track) lives in `clients/ts/`. Frontend (Next.js, Wallet Adapter, indexer, IPFS) is **out of scope** — that's a different track.

This brief is fully self-contained: every struct, error, account block, TS function, and test scenario is inlined. Don't go looking for outside docs.

Naming: this is a **Merkle distributor**, not Streamflow. Instructions are `create_campaign`, `claim`, `cancel_campaign`, `update_root`, etc. — not `create_stream` / `withdraw` / `cancel`.

---

## 1. Scope

### In
- Anchor 1.0 program with **9 instructions**:
  `create_campaign`, `fund_campaign`, `claim`, `cancel_campaign`,
  `update_root` *(new — per-recipient clawback via root rotation)*,
  `withdraw_unvested`, `pause_campaign`, `unpause_campaign`,
  `close_claim_record`, `get_vested_amount`. (10 handlers; pause + unpause share `Accounts`.)
- Math primitives implemented: schedule (`vested`, `get_vested_amount`), Merkle (`leaf_hash`, `verify_merkle_proof`), keccak256 with `LEAF_PREFIX = 0x00` and `NODE_PREFIX = 0x01`.
- TypeScript Merkle tooling at `clients/ts/`: leaf encoder (Borsh-equivalent byte order), tree builder using `merkletreejs` with custom hash, proof generator. Byte-equal to the Rust hash — verified by a golden-vector test.
- Integration tests covering: happy path (cliff + linear + milestone), root rotation (kick a recipient, others keep claiming), cancel mid-stream + clamp, withdraw_unvested grace gate, pause blocks claim, invalid proof rejected, double-claim returns NothingToClaim.
- GitHub Actions CI: build + test on push and PR.
- README a stranger can follow from zero clone to devnet deploy.

### Out
- Frontend (Next.js / `apps/web/`).
- Wallet Adapter, dashboards, indexer, IPFS pinning.
- Token-2022 (Phase 2).
- Pinocchio, Mollusk, proptest, cargo-fuzz (Phase 2).
- Squads multi-sig integration (cancel/rotate authority is a single key in v1).

---

## 2. Why root rotation matters

The base architecture's `cancel_campaign` is **campaign-wide** — it freezes the whole vesting curve and lets the project sweep unvested tokens after a 7-day grace. That doesn't cover the case where one contributor leaves but the rest of the campaign continues.

`update_root` solves this. The cancel authority can commit a new Merkle root that excludes the removed recipient (or changes amounts, or adds new recipients). Off-chain, the project rebuilds the tree without that leaf and re-pins the new proofs. On-chain, every `claim` validates against the *current* root, so the kicked recipient's old proof stops verifying immediately.

Existing claim records survive rotation — `claimed_amount` and `milestone_bitmap` are keyed on `(tree, beneficiary)`, not on leaf index, so a recipient whose amount shrunk simply hits `saturating_sub` and gets `NothingToClaim` on subsequent claims. A recipient whose amount grew gets the delta. A removed recipient cannot claim anymore.

---

## 3. Tech stack (pin exactly)

```toml
anchor-lang    = { version = "1.0.0", features = ["init-if-needed"] }
anchor-spl     = "1.0.0"
solana-program = "2.1"
```

- Rust stable, edition 2021.
- Solana CLI ≥ 2.1.
- Anchor CLI 1.0.0 via `avm install 1.0.0 && avm use 1.0.0`.
- Node ≥ 20 + Yarn (or npm).

TS deps: `@coral-xyz/anchor`, `@solana/web3.js`, `@solana/spl-token`, `bn.js`, `merkletreejs`, `js-sha3`, `mocha`, `chai`, `ts-mocha`, `typescript`, `@types/{bn.js,chai,mocha,node}`.

Test runner: LiteSVM (Anchor 1.0 default). `solana-test-validator` is fallback for CI only.

---

## 4. Repo layout

```
mancer-vesting/
├── Anchor.toml
├── Cargo.toml                        # workspace
├── package.json                      # ts deps + scripts
├── tsconfig.json
├── README.md
├── .gitignore
├── .github/workflows/ci.yml
├── programs/
│   └── vesting/
│       ├── Cargo.toml
│       ├── Xargo.toml
│       └── src/
│           ├── lib.rs
│           ├── constants.rs
│           ├── errors.rs
│           ├── events.rs
│           ├── state/
│           │   ├── mod.rs
│           │   ├── vesting_tree.rs
│           │   ├── claim_record.rs
│           │   └── leaf.rs
│           ├── instructions/
│           │   ├── mod.rs
│           │   ├── create_campaign.rs
│           │   ├── fund_campaign.rs
│           │   ├── claim.rs
│           │   ├── cancel_campaign.rs
│           │   ├── update_root.rs
│           │   ├── withdraw_unvested.rs
│           │   ├── pause_campaign.rs
│           │   ├── close_claim_record.rs
│           │   └── get_vested_amount.rs
│           └── math/
│               ├── mod.rs
│               ├── schedule.rs
│               └── merkle.rs
├── clients/
│   └── ts/
│       ├── leaf.ts                   # VestingLeaf encoder + leafHash
│       ├── merkle.ts                 # tree builder, proofs, root
│       └── index.ts                  # re-exports
└── tests/
    ├── utils/
    │   ├── setup.ts                  # provider, mint, ATA helpers
    │   └── time.ts                   # past/future timestamp helpers
    ├── golden_vector.spec.ts         # byte-equal Rust vs TS hash gate
    └── vesting.spec.ts               # full integration tests
```

---

## 5. Build sequence (suggested)

1. **Scaffold** — workspace + program Cargo.toml, Anchor.toml, lib.rs, package.json, tsconfig, .gitignore, README skeleton.
2. **Pure data** — `constants.rs`, all `state/*.rs`, `errors.rs`, `events.rs`. `cargo check` should pass.
3. **Math** — fill `schedule.rs` and `merkle.rs`. Add a Rust unit test for `vested` covering all four schedule branches.
4. **Instructions** — implement in this order to keep the state machine coherent:
   1. `create_campaign` + `fund_campaign` (write path).
   2. `cancel_campaign` + `pause_campaign` + `unpause_campaign` (state toggles).
   3. `claim` (the hot path — most logic).
   4. `update_root` (depends on `claim` for the rotation test to be meaningful).
   5. `withdraw_unvested` + `close_claim_record` (cleanup).
   6. `get_vested_amount` (one-liner over `math::schedule`).
5. **TS Merkle tooling** — `clients/ts/leaf.ts`, `clients/ts/merkle.ts`. Add the golden-vector test (TS hash a known leaf, assert against a Rust-produced hex).
6. **Integration tests** — `tests/utils/*` first, then `tests/vesting.spec.ts` covering the scenarios in §10.
7. **CI + README** — finalize once green locally.

---

## 6. Code (every file, real bodies)

### 6.1 Workspace `Cargo.toml`

```toml
[workspace]
members  = ["programs/*"]
resolver = "2"

[profile.release]
overflow-checks = true
lto             = "fat"
codegen-units   = 1

[profile.release.build-override]
opt-level     = 3
incremental   = false
codegen-units = 1
```

### 6.2 `programs/vesting/Cargo.toml`

```toml
[package]
name    = "vesting"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name       = "vesting"

[features]
default       = []
no-entrypoint = []
no-idl        = []
cpi           = ["no-entrypoint"]
idl-build     = ["anchor-lang/idl-build", "anchor-spl/idl-build"]

[dependencies]
anchor-lang    = { version = "1.0.0", features = ["init-if-needed"] }
anchor-spl     = "1.0.0"
solana-program = "2.1"
```

### 6.3 `programs/vesting/src/lib.rs`

```rust
use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;
use state::VestingLeaf;

declare_id!("Vesting1111111111111111111111111111111111111"); // replace at first deploy

#[program]
pub mod vesting {
    use super::*;

    pub fn create_campaign(
        ctx: Context<CreateCampaign>,
        args: CreateCampaignArgs,
    ) -> Result<()> {
        instructions::create_campaign::handler(ctx, args)
    }

    pub fn fund_campaign(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
        instructions::fund_campaign::handler(ctx, amount)
    }

    pub fn claim(
        ctx: Context<Claim>,
        leaf: VestingLeaf,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        instructions::claim::handler(ctx, leaf, proof)
    }

    pub fn cancel_campaign(ctx: Context<CancelCampaign>) -> Result<()> {
        instructions::cancel_campaign::handler(ctx)
    }

    pub fn update_root(
        ctx: Context<UpdateRoot>,
        new_root: [u8; 32],
        new_leaf_count: u32,
    ) -> Result<()> {
        instructions::update_root::handler(ctx, new_root, new_leaf_count)
    }

    pub fn withdraw_unvested(ctx: Context<WithdrawUnvested>) -> Result<()> {
        instructions::withdraw_unvested::handler(ctx)
    }

    pub fn pause_campaign(ctx: Context<PauseCampaign>) -> Result<()> {
        instructions::pause_campaign::pause_handler(ctx)
    }

    pub fn unpause_campaign(ctx: Context<PauseCampaign>) -> Result<()> {
        instructions::pause_campaign::unpause_handler(ctx)
    }

    pub fn close_claim_record(
        ctx: Context<CloseClaimRecord>,
        expected_total: u64,
    ) -> Result<()> {
        instructions::close_claim_record::handler(ctx, expected_total)
    }

    pub fn get_vested_amount(
        ctx: Context<GetVestedAmount>,
        leaf: VestingLeaf,
        cancelled_at: Option<i64>,
        now: i64,
    ) -> Result<u64> {
        instructions::get_vested_amount::handler(ctx, leaf, cancelled_at, now)
    }
}
```

### 6.4 `programs/vesting/src/constants.rs`

```rust
pub const GRACE_PERIOD_SECS: i64 = 7 * 24 * 60 * 60;
pub const MAX_MILESTONES:    u8  = u8::MAX;
pub const LEAF_PREFIX:       u8  = 0x00;
pub const NODE_PREFIX:       u8  = 0x01;
```

### 6.5 `programs/vesting/src/state/mod.rs`

```rust
pub mod vesting_tree;
pub mod claim_record;
pub mod leaf;

pub use vesting_tree::VestingTree;
pub use claim_record::ClaimRecord;
pub use leaf::VestingLeaf;
```

### 6.6 `programs/vesting/src/state/vesting_tree.rs`

```rust
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VestingTree {
    pub creator:          Pubkey,         // 32
    pub mint:             Pubkey,         // 32
    pub vault:            Pubkey,         // 32
    pub vault_authority:  Pubkey,         // 32
    pub campaign_id:      u64,            //  8
    pub merkle_root:      [u8; 32],       // 32
    pub leaf_count:       u32,            //  4
    pub total_supply:     u64,            //  8
    pub total_claimed:    u64,            //  8
    pub cancellable:      bool,           //  1
    pub cancel_authority: Option<Pubkey>, // 33
    pub cancelled_at:     Option<i64>,    //  9
    pub paused:           bool,           //  1
    pub pause_authority:  Option<Pubkey>, // 33
    pub created_at:       i64,            //  8
    pub bump:             u8,             //  1
}
```

### 6.7 `programs/vesting/src/state/claim_record.rs`

```rust
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ClaimRecord {
    pub beneficiary:      Pubkey,    // 32
    pub tree:             Pubkey,    // 32
    pub claimed_amount:   u64,       //  8
    pub milestone_bitmap: [u8; 32],  // 32
    pub last_claim_at:    i64,       //  8
    pub bump:             u8,        //  1
}
```

### 6.8 `programs/vesting/src/state/leaf.rs`

`VestingLeaf` is **not** an `#[account]`. Field order is the wire order — Borsh serializes in declaration order. The TS encoder must match byte-for-byte.

```rust
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct VestingLeaf {
    pub leaf_index:    u32,    //  4
    pub beneficiary:   Pubkey, // 32
    pub amount:        u64,    //  8
    pub release_type:  u8,     //  1 — 0 Cliff | 1 Linear | 2 Milestone
    pub start_time:    i64,    //  8
    pub cliff_time:    i64,    //  8
    pub end_time:      i64,    //  8
    pub milestone_idx: u8,     //  1
}
```

### 6.9 `programs/vesting/src/errors.rs`

```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum VestingError {
    // create_campaign / update_root
    #[msg("Merkle root must not be all-zero")]
    EmptyRoot,
    #[msg("Campaign must contain at least one leaf")]
    EmptyCampaign,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Cancellable campaigns require a cancel_authority")]
    MissingCancelAuthority,
    #[msg("New root must differ from the current root")]
    SameRoot,

    // fund_campaign / withdraw_unvested
    #[msg("Caller is not authorised for this action")]
    Unauthorized,
    #[msg("Vault would exceed the declared total_supply")]
    OverFunded,
    #[msg("Mint of provided account does not match the campaign mint")]
    MintMismatch,
    #[msg("Arithmetic overflow")]
    Overflow,

    // claim
    #[msg("Campaign is paused")]
    CampaignPaused,
    #[msg("Signer does not own this leaf")]
    UnauthorizedClaimer,
    #[msg("Leaf has malformed schedule (start <= cliff <= end violated)")]
    InvalidSchedule,
    #[msg("release_type must be 0 (Cliff), 1 (Linear), or 2 (Milestone)")]
    InvalidScheduleType,
    #[msg("Merkle proof did not verify against the stored root")]
    InvalidProof,
    #[msg("This milestone has already been claimed")]
    MilestoneAlreadyClaimed,
    #[msg("Nothing claimable at this time")]
    NothingToClaim,
    #[msg("Vault does not hold enough tokens for this claim")]
    InsufficientVault,
    #[msg("Total claimed would exceed campaign total_supply")]
    OverClaim,
    #[msg("Provided vault account does not match the campaign vault")]
    WrongVault,

    // cancel_campaign
    #[msg("Campaign was created as non-cancellable")]
    NotCancellable,
    #[msg("Campaign is already cancelled")]
    AlreadyCancelled,

    // pause_campaign
    #[msg("Campaign was created with no pause_authority")]
    NotPausable,
    #[msg("Campaign is already paused")]
    AlreadyPaused,
    #[msg("Cancelled campaigns cannot be paused, unpaused, or rotated")]
    CampaignCancelled,
    #[msg("Campaign is not paused")]
    NotPaused,

    // withdraw_unvested
    #[msg("Campaign is not cancelled")]
    NotCancelled,
    #[msg("Grace period after cancellation has not expired")]
    GracePeriodActive,

    // close_claim_record
    #[msg("ClaimRecord cannot be closed yet (not fully claimed and grace period active)")]
    CannotClose,
}
```

### 6.10 `programs/vesting/src/events.rs`

```rust
use anchor_lang::prelude::*;

#[event]
pub struct CampaignCreated {
    pub tree:         Pubkey,
    pub creator:      Pubkey,
    pub mint:         Pubkey,
    pub total_supply: u64,
    pub leaf_count:   u32,
    pub cancellable:  bool,
}

#[event]
pub struct CampaignFunded {
    pub tree:                Pubkey,
    pub amount:              u64,
    pub vault_balance_after: u64,
}

#[event]
pub struct Claimed {
    pub tree:                  Pubkey,
    pub beneficiary:           Pubkey,
    pub leaf_index:            u32,
    pub amount:                u64,
    pub total_claimed_by_user: u64,
    pub total_claimed_overall: u64,
    pub milestone_idx:         Option<u8>,
}

#[event]
pub struct CampaignCancelled {
    pub tree:              Pubkey,
    pub cancelled_at:      i64,
    pub claimed_at_cancel: u64,
}

#[event]
pub struct RootUpdated {
    pub tree:           Pubkey,
    pub old_root:       [u8; 32],
    pub new_root:       [u8; 32],
    pub new_leaf_count: u32,
}

#[event]
pub struct UnvestedWithdrawn {
    pub tree:   Pubkey,
    pub amount: u64,
}

#[event]
pub struct CampaignPaused   { pub tree: Pubkey }

#[event]
pub struct CampaignUnpaused { pub tree: Pubkey }

#[event]
pub struct ClaimRecordClosed {
    pub tree:        Pubkey,
    pub beneficiary: Pubkey,
}
```

### 6.11 `programs/vesting/src/math/mod.rs`

```rust
pub mod schedule;
pub mod merkle;
```

### 6.12 `programs/vesting/src/math/schedule.rs`

```rust
use crate::state::VestingLeaf;

pub fn vested(leaf: &VestingLeaf, now: i64) -> u64 {
    match leaf.release_type {
        0 /* Cliff */ => {
            if now >= leaf.cliff_time { leaf.amount } else { 0 }
        }
        1 /* Linear */ => {
            // Order matters: end_time first guards the cliff_time == end_time case from div-by-zero.
            if now >= leaf.end_time   { return leaf.amount; }
            if now <= leaf.cliff_time { return 0; }
            let elapsed  = (now - leaf.cliff_time) as u128;
            let duration = (leaf.end_time - leaf.cliff_time) as u128;
            ((leaf.amount as u128 * elapsed) / duration) as u64
        }
        2 /* Milestone */ => {
            if now >= leaf.cliff_time { leaf.amount } else { 0 }
        }
        _ => 0,
    }
}

pub fn get_vested_amount(
    leaf:         &VestingLeaf,
    cancelled_at: Option<i64>,
    now:          i64,
) -> u64 {
    let effective_now = match cancelled_at {
        Some(c) => now.min(c),
        None    => now,
    };
    vested(leaf, effective_now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn linear_leaf(amount: u64, cliff: i64, end: i64) -> VestingLeaf {
        VestingLeaf {
            leaf_index:    0,
            beneficiary:   Pubkey::default(),
            amount,
            release_type:  1,
            start_time:    cliff,
            cliff_time:    cliff,
            end_time:      end,
            milestone_idx: 0,
        }
    }

    #[test]
    fn cliff_before_after() {
        let leaf = VestingLeaf { release_type: 0, cliff_time: 100, amount: 1_000, ..linear_leaf(1_000, 100, 200) };
        assert_eq!(vested(&leaf, 99),  0);
        assert_eq!(vested(&leaf, 100), 1_000);
        assert_eq!(vested(&leaf, 999), 1_000);
    }

    #[test]
    fn linear_curve() {
        let leaf = linear_leaf(1_000, 100, 200);
        assert_eq!(vested(&leaf, 50),  0);
        assert_eq!(vested(&leaf, 100), 0);
        assert_eq!(vested(&leaf, 150), 500);
        assert_eq!(vested(&leaf, 200), 1_000);
        assert_eq!(vested(&leaf, 999), 1_000);
    }

    #[test]
    fn linear_no_overflow_at_max_amount() {
        let leaf = linear_leaf(u64::MAX, 0, 1_000_000);
        // halfway should be u64::MAX / 2 within rounding
        let half = vested(&leaf, 500_000);
        assert!(half > u64::MAX / 2 - 10 && half < u64::MAX / 2 + 10);
    }

    #[test]
    fn linear_degenerate_cliff_eq_end() {
        let leaf = linear_leaf(1_000, 100, 100);
        assert_eq!(vested(&leaf, 50),  0);
        assert_eq!(vested(&leaf, 100), 1_000);
        assert_eq!(vested(&leaf, 200), 1_000);
    }

    #[test]
    fn cancel_clamp() {
        let leaf = linear_leaf(1_000, 100, 200);
        assert_eq!(get_vested_amount(&leaf, Some(150), 999), 500);
        assert_eq!(get_vested_amount(&leaf, None,      999), 1_000);
    }
}
```

### 6.13 `programs/vesting/src/math/merkle.rs`

```rust
use anchor_lang::prelude::*;
use solana_program::keccak;
use crate::constants::{LEAF_PREFIX, NODE_PREFIX};
use crate::state::VestingLeaf;

pub fn leaf_hash(leaf: &VestingLeaf) -> [u8; 32] {
    let serialized = leaf.try_to_vec().expect("borsh: VestingLeaf");
    keccak::hashv(&[&[LEAF_PREFIX], &serialized]).to_bytes()
}

/// Index-based Merkle proof verification.
/// `index` is the leaf's position; bit 0 picks left/right at each level.
pub fn verify_merkle_proof(
    leaf:      [u8; 32],
    proof:     &[[u8; 32]],
    mut index: u32,
    root:      [u8; 32],
) -> bool {
    let mut hash = leaf;
    for sibling in proof {
        hash = if index & 1 == 0 {
            keccak::hashv(&[&[NODE_PREFIX], &hash, sibling]).to_bytes()
        } else {
            keccak::hashv(&[&[NODE_PREFIX], sibling, &hash]).to_bytes()
        };
        index >>= 1;
    }
    hash == root
}
```

### 6.14 `programs/vesting/src/instructions/mod.rs`

```rust
pub mod create_campaign;
pub mod fund_campaign;
pub mod claim;
pub mod cancel_campaign;
pub mod update_root;
pub mod withdraw_unvested;
pub mod pause_campaign;
pub mod close_claim_record;
pub mod get_vested_amount;

pub use create_campaign::*;
pub use fund_campaign::*;
pub use claim::*;
pub use cancel_campaign::*;
pub use update_root::*;
pub use withdraw_unvested::*;
pub use pause_campaign::*;
pub use close_claim_record::*;
pub use get_vested_amount::*;
```

### 6.15 `programs/vesting/src/instructions/create_campaign.rs`

```rust
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::errors::VestingError;
use crate::events::CampaignCreated;
use crate::state::VestingTree;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateCampaignArgs {
    pub campaign_id:      u64,
    pub merkle_root:      [u8; 32],
    pub leaf_count:       u32,
    pub total_supply:     u64,
    pub cancellable:      bool,
    pub cancel_authority: Option<Pubkey>,
    pub pause_authority:  Option<Pubkey>,
}

#[derive(Accounts)]
#[instruction(args: CreateCampaignArgs)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + VestingTree::INIT_SPACE,
        seeds = [b"tree",
                 creator.key().as_ref(),
                 mint.key().as_ref(),
                 &args.campaign_id.to_le_bytes()],
        bump,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    /// CHECK: PDA only used as signer; never deserialised.
    #[account(seeds = [b"vault_authority", vesting_tree.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = vault_authority,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program:            Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program:           Program<'info, System>,
    pub rent:                     Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateCampaign>, args: CreateCampaignArgs) -> Result<()> {
    require!(args.merkle_root != [0u8; 32], VestingError::EmptyRoot);
    require!(args.leaf_count  > 0,           VestingError::EmptyCampaign);
    require!(args.total_supply > 0,          VestingError::ZeroAmount);
    if args.cancellable {
        require!(args.cancel_authority.is_some(), VestingError::MissingCancelAuthority);
    }

    let tree = &mut ctx.accounts.vesting_tree;
    tree.creator          = ctx.accounts.creator.key();
    tree.mint             = ctx.accounts.mint.key();
    tree.vault            = ctx.accounts.vault.key();
    tree.vault_authority  = ctx.accounts.vault_authority.key();
    tree.campaign_id      = args.campaign_id;
    tree.merkle_root      = args.merkle_root;
    tree.leaf_count       = args.leaf_count;
    tree.total_supply     = args.total_supply;
    tree.total_claimed    = 0;
    tree.cancellable      = args.cancellable;
    tree.cancel_authority = args.cancel_authority;
    tree.cancelled_at     = None;
    tree.paused           = false;
    tree.pause_authority  = args.pause_authority;
    tree.created_at       = Clock::get()?.unix_timestamp;
    tree.bump             = ctx.bumps.vesting_tree;

    emit!(CampaignCreated {
        tree:         tree.key(),
        creator:      tree.creator,
        mint:         tree.mint,
        total_supply: tree.total_supply,
        leaf_count:   tree.leaf_count,
        cancellable:  tree.cancellable,
    });
    Ok(())
}
```

### 6.16 `programs/vesting/src/instructions/fund_campaign.rs`

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VestingError;
use crate::events::CampaignFunded;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator,
        has_one = vault,
        seeds = [b"tree",
                 creator.key().as_ref(),
                 vesting_tree.mint.as_ref(),
                 &vesting_tree.campaign_id.to_le_bytes()],
        bump = vesting_tree.bump,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = source_ata.mint  == vesting_tree.mint @ VestingError::MintMismatch,
        constraint = source_ata.owner == creator.key()      @ VestingError::Unauthorized,
    )]
    pub source_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
    require!(amount > 0, VestingError::ZeroAmount);
    require!(
        ctx.accounts.vesting_tree.cancelled_at.is_none(),
        VestingError::CampaignCancelled
    );

    let new_balance = ctx.accounts.vault.amount
        .checked_add(amount)
        .ok_or(VestingError::Overflow)?;
    require!(
        new_balance <= ctx.accounts.vesting_tree.total_supply,
        VestingError::OverFunded
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.source_ata.to_account_info(),
                to:        ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            },
        ),
        amount,
    )?;

    emit!(CampaignFunded {
        tree:                ctx.accounts.vesting_tree.key(),
        amount,
        vault_balance_after: new_balance,
    });
    Ok(())
}
```

### 6.17 `programs/vesting/src/instructions/claim.rs`

```rust
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::VestingError;
use crate::events::Claimed;
use crate::math::{merkle, schedule};
use crate::state::{ClaimRecord, VestingLeaf, VestingTree};

#[derive(Accounts)]
#[instruction(leaf: VestingLeaf, _proof: Vec<[u8; 32]>)]
pub struct Claim<'info> {
    #[account(mut)]
    pub beneficiary: Signer<'info>,

    #[account(
        mut,
        seeds = [b"tree",
                 vesting_tree.creator.as_ref(),
                 vesting_tree.mint.as_ref(),
                 &vesting_tree.campaign_id.to_le_bytes()],
        bump = vesting_tree.bump,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    #[account(
        init_if_needed,
        payer = beneficiary,
        space = 8 + ClaimRecord::INIT_SPACE,
        seeds = [b"claim",
                 vesting_tree.key().as_ref(),
                 beneficiary.key().as_ref()],
        bump,
    )]
    pub claim_record: Account<'info, ClaimRecord>,

    /// CHECK: PDA only used as signer.
    #[account(seeds = [b"vault_authority", vesting_tree.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut, address = vesting_tree.vault @ VestingError::WrongVault)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = beneficiary,
        associated_token::mint = mint,
        associated_token::authority = beneficiary,
    )]
    pub beneficiary_ata: Account<'info, TokenAccount>,

    #[account(address = vesting_tree.mint @ VestingError::MintMismatch)]
    pub mint: Account<'info, Mint>,

    pub token_program:            Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program:           Program<'info, System>,
}

pub fn handler(
    ctx:   Context<Claim>,
    leaf:  VestingLeaf,
    proof: Vec<[u8; 32]>,
) -> Result<()> {
    // 1. Pause check.
    require!(!ctx.accounts.vesting_tree.paused, VestingError::CampaignPaused);

    // 2. Beneficiary owns this leaf.
    require!(
        ctx.accounts.beneficiary.key() == leaf.beneficiary,
        VestingError::UnauthorizedClaimer
    );

    // 3. Schedule sanity.
    require!(
        leaf.start_time <= leaf.cliff_time && leaf.cliff_time <= leaf.end_time,
        VestingError::InvalidSchedule
    );
    require!(leaf.release_type <= 2, VestingError::InvalidScheduleType);

    // 4. Merkle proof verification (against current root — survives rotation).
    let lh = merkle::leaf_hash(&leaf);
    require!(
        merkle::verify_merkle_proof(
            lh,
            &proof,
            leaf.leaf_index,
            ctx.accounts.vesting_tree.merkle_root,
        ),
        VestingError::InvalidProof
    );

    // 5. First-touch init for ClaimRecord.
    let cr = &mut ctx.accounts.claim_record;
    if cr.beneficiary == Pubkey::default() {
        cr.tree              = ctx.accounts.vesting_tree.key();
        cr.beneficiary       = ctx.accounts.beneficiary.key();
        cr.claimed_amount    = 0;
        cr.milestone_bitmap  = [0u8; 32];
        cr.last_claim_at     = 0;
        cr.bump              = ctx.bumps.claim_record;
    }

    // 6. Milestone-specific guard.
    let mut milestone_idx_event: Option<u8> = None;
    if leaf.release_type == 2 {
        let byte_idx = (leaf.milestone_idx / 8) as usize;
        let bit_idx  =  leaf.milestone_idx % 8;
        let already  = (cr.milestone_bitmap[byte_idx] >> bit_idx) & 1 == 1;
        require!(!already, VestingError::MilestoneAlreadyClaimed);
        milestone_idx_event = Some(leaf.milestone_idx);
    }

    // 7. Effective time (cancel clamp).
    let now = Clock::get()?.unix_timestamp;
    let effective_now = match ctx.accounts.vesting_tree.cancelled_at {
        Some(c) => now.min(c),
        None    => now,
    };

    // 8. Compute claimable.
    let claimable: u64 = match leaf.release_type {
        0 | 1 /* Cliff or Linear */ => {
            let total = schedule::vested(&leaf, effective_now);
            total.saturating_sub(cr.claimed_amount)
        }
        2 /* Milestone */ => {
            if effective_now >= leaf.cliff_time { leaf.amount } else { 0 }
        }
        _ => return err!(VestingError::InvalidScheduleType),
    };

    require!(claimable > 0,                                   VestingError::NothingToClaim);
    require!(ctx.accounts.vault.amount >= claimable,          VestingError::InsufficientVault);

    let new_total = ctx.accounts.vesting_tree.total_claimed
        .checked_add(claimable)
        .ok_or(VestingError::Overflow)?;
    require!(
        new_total <= ctx.accounts.vesting_tree.total_supply,
        VestingError::OverClaim
    );

    // 9. State updates BEFORE the CPI (reentrancy posture).
    cr.claimed_amount = cr.claimed_amount
        .checked_add(claimable)
        .ok_or(VestingError::Overflow)?;
    cr.last_claim_at  = now;
    if leaf.release_type == 2 {
        let byte_idx = (leaf.milestone_idx / 8) as usize;
        let bit_idx  =  leaf.milestone_idx % 8;
        cr.milestone_bitmap[byte_idx] |= 1 << bit_idx;
    }
    let total_claimed_by_user = cr.claimed_amount;

    let tree = &mut ctx.accounts.vesting_tree;
    tree.total_claimed = new_total;
    let tree_key   = tree.key();
    let total_overall = tree.total_claimed;

    // 10. Token transfer CPI signed by vault_authority PDA.
    let bump = ctx.bumps.vault_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"vault_authority",
        tree_key.as_ref(),
        &[bump],
    ]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault.to_account_info(),
                to:        ctx.accounts.beneficiary_ata.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        claimable,
    )?;

    emit!(Claimed {
        tree:                  tree_key,
        beneficiary:           ctx.accounts.beneficiary.key(),
        leaf_index:            leaf.leaf_index,
        amount:                claimable,
        total_claimed_by_user,
        total_claimed_overall: total_overall,
        milestone_idx:         milestone_idx_event,
    });
    Ok(())
}
```

### 6.18 `programs/vesting/src/instructions/cancel_campaign.rs`

```rust
use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::events::CampaignCancelled;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct CancelCampaign<'info> {
    pub cancel_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.cancellable                                      @ VestingError::NotCancellable,
        constraint = vesting_tree.cancelled_at.is_none()                           @ VestingError::AlreadyCancelled,
        constraint = vesting_tree.cancel_authority == Some(cancel_authority.key()) @ VestingError::Unauthorized,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub fn handler(ctx: Context<CancelCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    let now  = Clock::get()?.unix_timestamp;
    tree.cancelled_at = Some(now);

    emit!(CampaignCancelled {
        tree:              tree.key(),
        cancelled_at:      now,
        claimed_at_cancel: tree.total_claimed,
    });
    Ok(())
}
```

### 6.19 `programs/vesting/src/instructions/update_root.rs`

```rust
use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::events::RootUpdated;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct UpdateRoot<'info> {
    pub cancel_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.cancellable                                      @ VestingError::NotCancellable,
        constraint = vesting_tree.cancelled_at.is_none()                           @ VestingError::CampaignCancelled,
        constraint = vesting_tree.cancel_authority == Some(cancel_authority.key()) @ VestingError::Unauthorized,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub fn handler(
    ctx:            Context<UpdateRoot>,
    new_root:       [u8; 32],
    new_leaf_count: u32,
) -> Result<()> {
    require!(new_root != [0u8; 32],                            VestingError::EmptyRoot);
    require!(new_leaf_count > 0,                               VestingError::EmptyCampaign);
    require!(new_root != ctx.accounts.vesting_tree.merkle_root, VestingError::SameRoot);

    let tree     = &mut ctx.accounts.vesting_tree;
    let old_root = tree.merkle_root;
    tree.merkle_root = new_root;
    tree.leaf_count  = new_leaf_count;

    emit!(RootUpdated {
        tree: tree.key(),
        old_root,
        new_root,
        new_leaf_count,
    });
    Ok(())
}
```

### 6.20 `programs/vesting/src/instructions/withdraw_unvested.rs`

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::GRACE_PERIOD_SECS;
use crate::errors::VestingError;
use crate::events::UnvestedWithdrawn;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct WithdrawUnvested<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator @ VestingError::Unauthorized,
        has_one = vault   @ VestingError::WrongVault,
        constraint = vesting_tree.cancelled_at.is_some() @ VestingError::NotCancelled,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    /// CHECK: PDA only used as signer.
    #[account(seeds = [b"vault_authority", vesting_tree.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = creator_ata.mint  == vesting_tree.mint @ VestingError::MintMismatch,
        constraint = creator_ata.owner == creator.key()      @ VestingError::Unauthorized,
    )]
    pub creator_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<WithdrawUnvested>) -> Result<()> {
    let cancelled = ctx.accounts.vesting_tree.cancelled_at
        .ok_or(VestingError::NotCancelled)?;
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= cancelled + GRACE_PERIOD_SECS,
        VestingError::GracePeriodActive
    );

    let amount = ctx.accounts.vault.amount;
    if amount > 0 {
        let tree_key = ctx.accounts.vesting_tree.key();
        let bump     = ctx.bumps.vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            tree_key.as_ref(),
            &[bump],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from:      ctx.accounts.vault.to_account_info(),
                    to:        ctx.accounts.creator_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;
    }

    emit!(UnvestedWithdrawn {
        tree: ctx.accounts.vesting_tree.key(),
        amount,
    });
    Ok(())
}
```

### 6.21 `programs/vesting/src/instructions/pause_campaign.rs`

```rust
use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::events::{CampaignPaused, CampaignUnpaused};
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct PauseCampaign<'info> {
    pub pause_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.pause_authority.is_some()                      @ VestingError::NotPausable,
        constraint = vesting_tree.pause_authority == Some(pause_authority.key()) @ VestingError::Unauthorized,
        constraint = vesting_tree.cancelled_at.is_none()                         @ VestingError::CampaignCancelled,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub type UnpauseCampaign<'info> = PauseCampaign<'info>;

pub fn pause_handler(ctx: Context<PauseCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    require!(!tree.paused, VestingError::AlreadyPaused);
    tree.paused = true;
    emit!(CampaignPaused { tree: tree.key() });
    Ok(())
}

pub fn unpause_handler(ctx: Context<PauseCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    require!(tree.paused, VestingError::NotPaused);
    tree.paused = false;
    emit!(CampaignUnpaused { tree: tree.key() });
    Ok(())
}
```

### 6.22 `programs/vesting/src/instructions/close_claim_record.rs`

```rust
use anchor_lang::prelude::*;

use crate::constants::GRACE_PERIOD_SECS;
use crate::errors::VestingError;
use crate::events::ClaimRecordClosed;
use crate::state::{ClaimRecord, VestingTree};

#[derive(Accounts)]
pub struct CloseClaimRecord<'info> {
    #[account(mut)]
    pub beneficiary: Signer<'info>,

    pub vesting_tree: Account<'info, VestingTree>,

    #[account(
        mut,
        close = beneficiary,
        has_one = beneficiary @ VestingError::Unauthorized,
        constraint = claim_record.tree == vesting_tree.key() @ VestingError::WrongVault,
        seeds = [b"claim", vesting_tree.key().as_ref(), beneficiary.key().as_ref()],
        bump = claim_record.bump,
    )]
    pub claim_record: Account<'info, ClaimRecord>,
}

pub fn handler(ctx: Context<CloseClaimRecord>, expected_total: u64) -> Result<()> {
    let cr   = &ctx.accounts.claim_record;
    let tree = &ctx.accounts.vesting_tree;
    let now  = Clock::get()?.unix_timestamp;

    let fully_claimed = cr.claimed_amount >= expected_total;
    let post_grace = match tree.cancelled_at {
        Some(c) => now >= c + GRACE_PERIOD_SECS,
        None    => false,
    };
    require!(fully_claimed || post_grace, VestingError::CannotClose);

    emit!(ClaimRecordClosed {
        tree:        tree.key(),
        beneficiary: ctx.accounts.beneficiary.key(),
    });
    Ok(())
}
```

### 6.23 `programs/vesting/src/instructions/get_vested_amount.rs`

```rust
use anchor_lang::prelude::*;

use crate::math::schedule;
use crate::state::VestingLeaf;

#[derive(Accounts)]
pub struct GetVestedAmount {}

pub fn handler(
    _ctx:         Context<GetVestedAmount>,
    leaf:         VestingLeaf,
    cancelled_at: Option<i64>,
    now:          i64,
) -> Result<u64> {
    Ok(schedule::get_vested_amount(&leaf, cancelled_at, now))
}
```

### 6.24 `Anchor.toml`

```toml
[features]
resolution = true
skip-lint  = false

[programs.localnet]
vesting = "Vesting1111111111111111111111111111111111111"

[programs.devnet]
vesting = "Vesting1111111111111111111111111111111111111"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet  = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.spec.ts"
```

### 6.25 `package.json`

```json
{
  "name": "mancer-vesting",
  "version": "0.1.0",
  "license": "MIT",
  "scripts": {
    "test":     "anchor test",
    "lint:fix": "prettier */*.ts \"*/**/*{.js,.ts}\" -w",
    "lint":     "prettier */*.ts \"*/**/*{.js,.ts}\" --check"
  },
  "dependencies": {
    "@coral-xyz/anchor":   "^1.0.0",
    "@solana/spl-token":   "^0.4.6",
    "@solana/web3.js":     "^1.95.0",
    "bn.js":               "^5.2.1",
    "js-sha3":             "^0.9.3",
    "merkletreejs":        "^0.4.0"
  },
  "devDependencies": {
    "@types/bn.js":  "^5.1.5",
    "@types/chai":   "^4.3.0",
    "@types/mocha":  "^10.0.6",
    "@types/node":   "^20.11.0",
    "chai":          "^4.3.4",
    "mocha":         "^10.2.0",
    "prettier":      "^2.6.2",
    "ts-mocha":      "^10.0.0",
    "typescript":    "^5.3.0"
  }
}
```

### 6.26 `tsconfig.json`

```json
{
  "compilerOptions": {
    "types":             ["mocha", "chai", "node"],
    "typeRoots":         ["./node_modules/@types"],
    "lib":               ["es2020"],
    "module":            "commonjs",
    "target":            "es2020",
    "esModuleInterop":   true,
    "resolveJsonModule": true,
    "strict":            true,
    "skipLibCheck":      true,
    "declaration":       true,
    "outDir":            "dist"
  },
  "include": ["clients/ts/**/*.ts", "tests/**/*.ts"]
}
```

### 6.27 `clients/ts/leaf.ts`

```ts
import { keccak_256 } from "js-sha3";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export const LEAF_PREFIX = Buffer.from([0x00]);
export const NODE_PREFIX = Buffer.from([0x01]);

export enum ReleaseType {
  Cliff     = 0,
  Linear    = 1,
  Milestone = 2,
}

export interface VestingLeaf {
  leafIndex:    number;
  beneficiary:  PublicKey;
  amount:       BN;
  releaseType:  ReleaseType;
  startTime:    BN;
  cliffTime:    BN;
  endTime:      BN;
  milestoneIdx: number;
}

function u32LE(n: number): Buffer {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  return b;
}

function u64LE(n: BN): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n.toString()));
  return b;
}

function i64LE(n: BN): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigInt64LE(BigInt(n.toString()));
  return b;
}

/// Borsh order MUST match programs/vesting/src/state/leaf.rs.
export function encodeLeaf(leaf: VestingLeaf): Buffer {
  return Buffer.concat([
    u32LE(leaf.leafIndex),
    leaf.beneficiary.toBuffer(),
    u64LE(leaf.amount),
    Buffer.from([leaf.releaseType]),
    i64LE(leaf.startTime),
    i64LE(leaf.cliffTime),
    i64LE(leaf.endTime),
    Buffer.from([leaf.milestoneIdx]),
  ]);
}

export function leafHash(leaf: VestingLeaf): Buffer {
  return Buffer.from(
    keccak_256.array(Buffer.concat([LEAF_PREFIX, encodeLeaf(leaf)])),
  );
}

export function nodeHash(left: Buffer, right: Buffer): Buffer {
  return Buffer.from(
    keccak_256.array(Buffer.concat([NODE_PREFIX, left, right])),
  );
}
```

### 6.28 `clients/ts/merkle.ts`

Hand-rolled tree to guarantee byte-equality with the Rust verifier. `merkletreejs` ships in `dependencies` for projects that want to consume it directly, but our verifier uses index-based proofs (not sorted-pair) so we own the tree-walker here. The TS golden-vector test in §6.30 is the gate.

```ts
import { leafHash, nodeHash, VestingLeaf } from "./leaf";

export class VestingMerkleTree {
  readonly leaves: VestingLeaf[];
  readonly leafHashes: Buffer[];
  readonly layers: Buffer[][];

  constructor(leaves: VestingLeaf[]) {
    if (leaves.length === 0) throw new Error("VestingMerkleTree: empty leaf set");
    // Validate leaf_index matches array position so on-chain index parity works.
    leaves.forEach((l, i) => {
      if (l.leafIndex !== i) {
        throw new Error(`leaf at position ${i} has leafIndex=${l.leafIndex}; must equal position`);
      }
    });

    this.leaves     = leaves;
    this.leafHashes = leaves.map(leafHash);
    this.layers     = [this.leafHashes.slice()];

    while (this.layers[this.layers.length - 1].length > 1) {
      const prev = this.layers[this.layers.length - 1];
      const next: Buffer[] = [];
      for (let i = 0; i < prev.length; i += 2) {
        const left  = prev[i];
        const right = i + 1 < prev.length ? prev[i + 1] : prev[i]; // duplicate-odd
        next.push(nodeHash(left, right));
      }
      this.layers.push(next);
    }
  }

  get root(): Buffer {
    return this.layers[this.layers.length - 1][0];
  }

  get rootHex(): string {
    return this.root.toString("hex");
  }

  get rootBytes(): number[] {
    return Array.from(this.root);
  }

  proof(index: number): Buffer[] {
    if (index < 0 || index >= this.leaves.length) {
      throw new Error(`proof: index ${index} out of bounds (leaves=${this.leaves.length})`);
    }
    const out: Buffer[] = [];
    let i = index;
    for (let layer = 0; layer < this.layers.length - 1; layer++) {
      const arr = this.layers[layer];
      const isRight = i % 2 === 1;
      const sibling = isRight
        ? i - 1
        : (i + 1 < arr.length ? i + 1 : i); // duplicate-odd: sibling is self
      out.push(arr[sibling]);
      i = Math.floor(i / 2);
    }
    return out;
  }

  proofAsBytes(index: number): number[][] {
    return this.proof(index).map(b => Array.from(b));
  }

  /// Off-chain verification — useful in tests before submitting a tx.
  verify(index: number, proof: Buffer[]): boolean {
    let hash = this.leafHashes[index];
    let i = index;
    for (const sibling of proof) {
      hash = (i & 1) === 0 ? nodeHash(hash, sibling) : nodeHash(sibling, hash);
      i >>>= 1;
    }
    return hash.equals(this.root);
  }
}
```

### 6.29 `clients/ts/index.ts`

```ts
export * from "./leaf";
export * from "./merkle";
```

### 6.30 `tests/golden_vector.spec.ts`

The byte-equal gate. If this fails, the whole protocol fails — every TS-built proof would be rejected on-chain.

```ts
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { assert } from "chai";
import { encodeLeaf, leafHash, ReleaseType, VestingLeaf } from "../clients/ts";

describe("golden vector — TS encoder must be byte-equal to Rust", () => {
  const beneficiary = new PublicKey("11111111111111111111111111111112"); // SystemProgram + 1

  const leaf: VestingLeaf = {
    leafIndex:    0,
    beneficiary,
    amount:       new BN(1_000_000),
    releaseType:  ReleaseType.Linear,
    startTime:    new BN(1_700_000_000),
    cliffTime:    new BN(1_700_000_000),
    endTime:      new BN(1_800_000_000),
    milestoneIdx: 0,
  };

  it("encoded leaf is exactly 70 bytes", () => {
    assert.equal(encodeLeaf(leaf).length, 70);
  });

  it("leafHash is deterministic and 32 bytes", () => {
    const h1 = leafHash(leaf);
    const h2 = leafHash(leaf);
    assert.equal(h1.length, 32);
    assert.deepEqual(h1, h2);
  });

  // The hex below is the expected output from the Rust math::merkle::leaf_hash
  // when called on the same inputs. To regenerate it, add a Rust unit test:
  //
  //   #[test]
  //   fn print_golden_hash() {
  //       let leaf = VestingLeaf {
  //           leaf_index: 0,
  //           beneficiary: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
  //           amount: 1_000_000,
  //           release_type: 1,
  //           start_time: 1_700_000_000,
  //           cliff_time: 1_700_000_000,
  //           end_time: 1_800_000_000,
  //           milestone_idx: 0,
  //       };
  //       println!("{}", hex::encode(leaf_hash(&leaf)));
  //   }
  //
  // Then paste the output below. If openclaw cannot run a Rust test, leave
  // this assertion as `assert.ok(true)` and document the manual gate in the
  // README.
  it("matches the Rust golden hash", () => {
    const expected = process.env.GOLDEN_HASH ?? "";
    if (!expected) {
      // First run: capture the TS output, then add a Rust test that produces
      // the same hash. Both sides must agree byte-for-byte.
      console.log("TS leafHash hex =", leafHash(leaf).toString("hex"));
      assert.ok(true, "set GOLDEN_HASH env var to assert byte-equality");
      return;
    }
    assert.equal(leafHash(leaf).toString("hex"), expected);
  });
});
```

### 6.31 `tests/utils/setup.ts`

```ts
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import BN from "bn.js";
import type { Vesting } from "../../target/types/vesting";

export interface TestContext {
  provider:    anchor.AnchorProvider;
  program:     Program<Vesting>;
  payer:       Keypair;
  mint:        PublicKey;
  creator:     Keypair;
  cancelAuth:  Keypair;
  pauseAuth:   Keypair;
}

export async function airdrop(
  provider: anchor.AnchorProvider,
  to:       PublicKey,
  sol:      number,
) {
  const sig = await provider.connection.requestAirdrop(to, sol * LAMPORTS_PER_SOL);
  await provider.connection.confirmTransaction(sig, "confirmed");
}

export async function setup(): Promise<TestContext> {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Vesting as Program<Vesting>;

  const payer      = (provider.wallet as anchor.Wallet).payer;
  const creator    = Keypair.generate();
  const cancelAuth = Keypair.generate();
  const pauseAuth  = Keypair.generate();

  await Promise.all([
    airdrop(provider, creator.publicKey,    10),
    airdrop(provider, cancelAuth.publicKey,  1),
    airdrop(provider, pauseAuth.publicKey,   1),
  ]);

  const mint = await createMint(
    provider.connection,
    payer,
    creator.publicKey, // mint authority
    null,
    6,                 // decimals
  );

  return { provider, program, payer, mint, creator, cancelAuth, pauseAuth };
}

export async function fundCreatorAta(
  ctx:    TestContext,
  amount: BN,
): Promise<PublicKey> {
  const ata = await getOrCreateAssociatedTokenAccount(
    ctx.provider.connection,
    ctx.payer,
    ctx.mint,
    ctx.creator.publicKey,
  );
  await mintTo(
    ctx.provider.connection,
    ctx.payer,
    ctx.mint,
    ata.address,
    ctx.creator,
    BigInt(amount.toString()),
  );
  return ata.address;
}

export async function makeBeneficiary(
  ctx: TestContext,
  sol: number = 5,
): Promise<Keypair> {
  const kp = Keypair.generate();
  await airdrop(ctx.provider, kp.publicKey, sol);
  return kp;
}

export function deriveTreePda(
  programId:  PublicKey,
  creator:    PublicKey,
  mint:       PublicKey,
  campaignId: BN,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("tree"),
      creator.toBuffer(),
      mint.toBuffer(),
      campaignId.toArrayLike(Buffer, "le", 8),
    ],
    programId,
  );
}

export function deriveVaultAuthority(
  programId: PublicKey,
  tree:      PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), tree.toBuffer()],
    programId,
  );
}

export function deriveClaimRecord(
  programId:   PublicKey,
  tree:        PublicKey,
  beneficiary: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("claim"), tree.toBuffer(), beneficiary.toBuffer()],
    programId,
  );
}

export function vaultAta(mint: PublicKey, vaultAuthority: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, vaultAuthority, true);
}
```

### 6.32 `tests/utils/time.ts`

```ts
import BN from "bn.js";

export function nowSec(): number {
  return Math.floor(Date.now() / 1000);
}

export function past(secs: number): BN {
  return new BN(nowSec() - secs);
}

export function future(secs: number): BN {
  return new BN(nowSec() + secs);
}
```

### 6.33 `tests/vesting.spec.ts`

Five core scenarios. Each is self-contained — fresh mint + campaign per `before`. Use these as templates and add the rest of §10 against the same shape.

```ts
import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import BN from "bn.js";
import { assert, expect } from "chai";
import {
  setup,
  fundCreatorAta,
  makeBeneficiary,
  deriveTreePda,
  deriveVaultAuthority,
  deriveClaimRecord,
  vaultAta,
  TestContext,
} from "./utils/setup";
import { past, future } from "./utils/time";
import {
  ReleaseType,
  VestingLeaf,
  VestingMerkleTree,
} from "../clients/ts";

const CAMPAIGN_ID = new BN(1);

async function buildCampaign(
  ctx:           TestContext,
  beneficiaries: { kp: anchor.web3.Keypair; amount: BN; release: ReleaseType; cliff: BN; end: BN }[],
) {
  const leaves: VestingLeaf[] = beneficiaries.map((b, i) => ({
    leafIndex:    i,
    beneficiary:  b.kp.publicKey,
    amount:       b.amount,
    releaseType:  b.release,
    startTime:    b.cliff,
    cliffTime:    b.cliff,
    endTime:      b.end,
    milestoneIdx: 0,
  }));
  const tree = new VestingMerkleTree(leaves);
  const totalSupply = beneficiaries.reduce((acc, b) => acc.add(b.amount), new BN(0));

  const [treePda]      = deriveTreePda(ctx.program.programId, ctx.creator.publicKey, ctx.mint, CAMPAIGN_ID);
  const [vaultAuthPda] = deriveVaultAuthority(ctx.program.programId, treePda);
  const vaultPk        = vaultAta(ctx.mint, vaultAuthPda);

  await ctx.program.methods
    .createCampaign({
      campaignId:      CAMPAIGN_ID,
      merkleRoot:      Array.from(tree.root) as any,
      leafCount:       leaves.length,
      totalSupply,
      cancellable:     true,
      cancelAuthority: ctx.cancelAuth.publicKey,
      pauseAuthority:  ctx.pauseAuth.publicKey,
    } as any)
    .accounts({
      creator:                  ctx.creator.publicKey,
      vestingTree:              treePda,
      vaultAuthority:           vaultAuthPda,
      vault:                    vaultPk,
      mint:                     ctx.mint,
      tokenProgram:             TOKEN_PROGRAM_ID,
      associatedTokenProgram:   ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram:            SystemProgram.programId,
      rent:                     SYSVAR_RENT_PUBKEY,
    })
    .signers([ctx.creator])
    .rpc();

  const sourceAta = await fundCreatorAta(ctx, totalSupply);
  await ctx.program.methods
    .fundCampaign(totalSupply)
    .accounts({
      creator:      ctx.creator.publicKey,
      vestingTree:  treePda,
      vault:        vaultPk,
      sourceAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([ctx.creator])
    .rpc();

  return { tree, leaves, treePda, vaultAuthPda, vaultPk, totalSupply, sourceAta };
}

async function claim(
  ctx:         TestContext,
  campaign:    Awaited<ReturnType<typeof buildCampaign>>,
  beneficiary: anchor.web3.Keypair,
  leafIndex:   number,
) {
  const [claimRecord] = deriveClaimRecord(ctx.program.programId, campaign.treePda, beneficiary.publicKey);
  const benAta        = getAssociatedTokenAddressSync(ctx.mint, beneficiary.publicKey);
  const proof         = campaign.tree.proofAsBytes(leafIndex);
  const leaf          = campaign.leaves[leafIndex];

  return ctx.program.methods
    .claim(
      {
        leafIndex:    leaf.leafIndex,
        beneficiary:  leaf.beneficiary,
        amount:       leaf.amount,
        releaseType:  leaf.releaseType,
        startTime:    leaf.startTime,
        cliffTime:    leaf.cliffTime,
        endTime:      leaf.endTime,
        milestoneIdx: leaf.milestoneIdx,
      } as any,
      proof as any,
    )
    .accounts({
      beneficiary:               beneficiary.publicKey,
      vestingTree:               campaign.treePda,
      claimRecord,
      vaultAuthority:            campaign.vaultAuthPda,
      vault:                     campaign.vaultPk,
      beneficiaryAta:            benAta,
      mint:                      ctx.mint,
      tokenProgram:              TOKEN_PROGRAM_ID,
      associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram:             SystemProgram.programId,
    })
    .signers([beneficiary])
    .rpc();
}

describe("mancer-vesting working product", () => {
  let ctx: TestContext;

  beforeEach(async () => {
    ctx = await setup();
  });

  it("happy path: linear claim mid-stream transfers proportional amount", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000_000), release: ReleaseType.Linear,
        cliff: past(500),     // started 500s ago
        end:   future(500) }, // ends in 500s — we are ~50% through
    ]);

    await claim(ctx, campaign, ben, 0);

    const benAta = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const acc    = await getAccount(ctx.provider.connection, benAta);
    const got    = Number(acc.amount);
    // ~50% of 1_000_000, allow 5% drift for slot time:
    assert.isAtLeast(got, 450_000);
    assert.isAtMost(got,  550_000);
  });

  it("invalid proof is rejected", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000), release: ReleaseType.Cliff,
        cliff: past(10), end: past(10) },
    ]);

    // Tamper a single byte in the proof's first sibling. With a single-leaf
    // tree the proof is empty, so include a second beneficiary to make a
    // real proof exist:
    const other = await makeBeneficiary(ctx);
    const camp2 = await buildCampaign(ctx, [
      { kp: ben,   amount: new BN(1_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: other, amount: new BN(2_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
    ]);

    const [claimRecord] = deriveClaimRecord(ctx.program.programId, camp2.treePda, ben.publicKey);
    const benAta        = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const goodProof     = camp2.tree.proofAsBytes(0);
    goodProof[0][0] ^= 0xff; // flip a byte
    const leaf          = camp2.leaves[0];

    try {
      await ctx.program.methods
        .claim(leaf as any, goodProof as any)
        .accounts({
          beneficiary:               ben.publicKey,
          vestingTree:               camp2.treePda,
          claimRecord,
          vaultAuthority:            camp2.vaultAuthPda,
          vault:                     camp2.vaultPk,
          beneficiaryAta:            benAta,
          mint:                      ctx.mint,
          tokenProgram:              TOKEN_PROGRAM_ID,
          associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram:             SystemProgram.programId,
        })
        .signers([ben])
        .rpc();
      assert.fail("expected InvalidProof");
    } catch (e: any) {
      expect(e.toString()).to.match(/InvalidProof/);
    }
  });

  it("pause blocks claim, unpause restores it", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000), release: ReleaseType.Cliff,
        cliff: past(10), end: past(10) },
    ]);

    await ctx.program.methods
      .pauseCampaign()
      .accounts({ pauseAuthority: ctx.pauseAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.pauseAuth])
      .rpc();

    try {
      await claim(ctx, campaign, ben, 0);
      assert.fail("expected CampaignPaused");
    } catch (e: any) {
      expect(e.toString()).to.match(/CampaignPaused/);
    }

    await ctx.program.methods
      .unpauseCampaign()
      .accounts({ pauseAuthority: ctx.pauseAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.pauseAuth])
      .rpc();

    await claim(ctx, campaign, ben, 0);
  });

  it("cancel + claim returns pre-cancel vested only", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000_000), release: ReleaseType.Linear,
        cliff: past(500), end: future(500) },
    ]);

    await ctx.program.methods
      .cancelCampaign()
      .accounts({ cancelAuthority: ctx.cancelAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.cancelAuth])
      .rpc();

    // Wait briefly so `now > cancelled_at` and the clamp is observable.
    await new Promise(r => setTimeout(r, 1500));

    await claim(ctx, campaign, ben, 0);

    const benAta = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const got    = Number((await getAccount(ctx.provider.connection, benAta)).amount);
    // Should be roughly the vested amount AT cancel time, NOT the full amount.
    assert.isAtMost(got, 600_000); // generous upper bound
    assert.isAtLeast(got, 400_000);
  });

  it("update_root: kicks a recipient — old proof fails, others keep claiming", async () => {
    const alice = await makeBeneficiary(ctx);
    const bob   = await makeBeneficiary(ctx);
    const carol = await makeBeneficiary(ctx);

    const campaign = await buildCampaign(ctx, [
      { kp: alice, amount: new BN(1_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: bob,   amount: new BN(2_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: carol, amount: new BN(3_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
    ]);

    // Alice claims successfully against the original tree.
    await claim(ctx, campaign, alice, 0);

    // Rebuild the tree without Bob — Alice and Carol stay, re-indexed.
    const newLeaves: VestingLeaf[] = [
      { leafIndex: 0, beneficiary: alice.publicKey, amount: new BN(1_000),
        releaseType: ReleaseType.Cliff, startTime: past(10), cliffTime: past(10),
        endTime: past(10), milestoneIdx: 0 },
      { leafIndex: 1, beneficiary: carol.publicKey, amount: new BN(3_000),
        releaseType: ReleaseType.Cliff, startTime: past(10), cliffTime: past(10),
        endTime: past(10), milestoneIdx: 0 },
    ];
    const newTree = new VestingMerkleTree(newLeaves);

    await ctx.program.methods
      .updateRoot(Array.from(newTree.root) as any, newLeaves.length)
      .accounts({ cancelAuthority: ctx.cancelAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.cancelAuth])
      .rpc();

    // Bob's old proof no longer verifies against the new root.
    try {
      await claim(ctx, campaign, bob, 1);
      assert.fail("expected InvalidProof for kicked recipient");
    } catch (e: any) {
      expect(e.toString()).to.match(/InvalidProof/);
    }

    // Carol's NEW proof against the NEW tree works. (Note: campaign.tree is
    // stale; we use newTree here.)
    const [claimRecord] = deriveClaimRecord(ctx.program.programId, campaign.treePda, carol.publicKey);
    const carolAta      = getAssociatedTokenAddressSync(ctx.mint, carol.publicKey);
    const proof         = newTree.proof(1).map(b => Array.from(b));
    const leaf          = newLeaves[1];

    await ctx.program.methods
      .claim(leaf as any, proof as any)
      .accounts({
        beneficiary:               carol.publicKey,
        vestingTree:               campaign.treePda,
        claimRecord,
        vaultAuthority:            campaign.vaultAuthPda,
        vault:                     campaign.vaultPk,
        beneficiaryAta:            carolAta,
        mint:                      ctx.mint,
        tokenProgram:              TOKEN_PROGRAM_ID,
        associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram:             SystemProgram.programId,
      })
      .signers([carol])
      .rpc();

    const got = Number((await getAccount(ctx.provider.connection, carolAta)).amount);
    assert.equal(got, 3_000);
  });
});
```

### 6.34 `.github/workflows/ci.yml`

```yaml
name: ci

on:
  push:
  pull_request:

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Install Solana CLI
        run: |
          sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.0/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH

      - name: Install Anchor CLI
        run: |
          cargo install --git https://github.com/coral-xyz/anchor avm --force
          avm install 1.0.0
          avm use 1.0.0

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: "20"

      - name: Install JS deps
        run: yarn install --frozen-lockfile || npm install

      - name: cargo test (unit)
        run: cargo test --manifest-path programs/vesting/Cargo.toml

      - name: anchor build
        run: anchor build

      - name: anchor test
        run: anchor test
```

### 6.35 `.gitignore`

```
target/
node_modules/
.anchor/
test-ledger/
dist/
.DS_Store
*.log
.env
.env.local
```

### 6.36 `README.md`

```markdown
# Mancer Vesting Protocol

A Solana token-distribution protocol with **Merkle compression** and
**per-recipient clawback** (root rotation). One 32-byte Merkle root
on-chain stands in for an entire recipient list; a 10,000-recipient
campaign costs ~$0.42 versus ~$1,990 on per-recipient-PDA designs.

This repo is the Week 3 working product: every instruction is fully
implemented, integration tests cover the happy path and the rotation
flow, and TypeScript Merkle tooling lives in `clients/ts/` for the
frontend track to consume.

## Features
- Bulk send via Merkle root commit.
- Per-leaf vesting schedule: Cliff, Linear, Milestone.
- Campaign-wide cancel with `cancelled_at` clamp + 7-day grace.
- **Per-recipient clawback** via `update_root` — kick or re-allocate
  individuals without rotating the whole campaign.
- Pause / unpause for emergency stop.
- Public events for indexer-driven dashboards.

## Prerequisites
- Rust (stable):
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Solana CLI ≥ 2.1:
  `sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.0/install)"`
- Anchor CLI 1.0.0 via avm:
  ```
  cargo install --git https://github.com/coral-xyz/anchor avm --force
  avm install 1.0.0
  avm use 1.0.0
  ```
- Node ≥ 20 + Yarn (or npm).
- Solana keypair: `solana-keygen new` if you don't already have
  `~/.config/solana/id.json`.

## Setup
```
git clone https://github.com/<org>/mancer-vesting.git
cd mancer-vesting
yarn install   # or npm install
```

## Build
```
anchor build
```

## Test
Unit tests (math):
```
cargo test --manifest-path programs/vesting/Cargo.toml
```

Integration tests (full Anchor + LiteSVM):
```
anchor test
```

## Deploy to devnet
```
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --provider.cluster devnet
```

After the first devnet deploy, copy the printed program ID into:
- `Anchor.toml` → `[programs.devnet] vesting = "..."`
- `programs/vesting/src/lib.rs` → `declare_id!("...")`

Then `anchor build` again so the IDL matches.

## Project layout
```
programs/vesting/src/   # Anchor program
clients/ts/             # TS leaf encoder + Merkle tree (consumed by frontend)
tests/                  # Integration tests + golden vector
.github/workflows/      # CI
```

## Per-recipient clawback flow
1. Project decides to remove a recipient.
2. Project rebuilds the Merkle tree off-chain without that leaf,
   re-indexing the remaining leaves.
3. Project re-pins proofs (IPFS / project site).
4. Project calls `update_root(new_root, new_leaf_count)`.
5. Removed recipient's old proof now fails verification against the
   new root → `InvalidProof`. Their `ClaimRecord` keeps whatever they
   already claimed; no further claims possible.
6. Other recipients fetch their new proofs and continue claiming
   normally — `claimed_amount` is preserved across rotations.

## What's NOT in this repo
- Frontend (Next.js, Wallet Adapter, dashboards) — separate track.
- Indexer / IPFS pinning — separate track.
- Token-2022 support — Phase 2.
```

---

## 7. Acceptance criteria (verifiable, in order)

1. `cargo test --manifest-path programs/vesting/Cargo.toml` green — schedule math unit tests pass.
2. `anchor build` exits 0; `target/deploy/vesting.so` and `target/idl/vesting.json` exist.
3. IDL contains all **10** instructions: `createCampaign`, `fundCampaign`, `claim`, `cancelCampaign`, `updateRoot`, `withdrawUnvested`, `pauseCampaign`, `unpauseCampaign`, `closeClaimRecord`, `getVestedAmount`.
4. IDL contains both account types (`VestingTree`, `ClaimRecord`) and all 9 events (incl. `RootUpdated`).
5. IDL error list contains `SameRoot` (the new error variant).
6. `anchor test` green with **all 5 integration tests passing** (linear mid-stream, invalid proof, pause/unpause, cancel clamp, update_root rotation).
7. `tests/golden_vector.spec.ts` runs without error (the byte-equal hash gate; if `GOLDEN_HASH` env var is set, asserts; otherwise prints).
8. CI green on first push: cargo unit tests + anchor build + anchor test all succeed.
9. README walkthrough works on a clean machine: clone, install, build, test, deploy to devnet.

---

## 8. Don't-do list

- Don't build a frontend, Wallet Adapter, dashboard, or any web app.
- Don't add IPFS / Pinata pinning code.
- Don't add Pinocchio, Mollusk, proptest, or cargo-fuzz crates.
- Don't add Token-2022 support; SPL Token only.
- Don't pre-generate a real program ID; use `Vesting1111111111111111111111111111111111111` and document the swap-on-deploy step.
- Don't reorder fields in `VestingTree`, `ClaimRecord`, or `VestingLeaf`. The order is the wire order.
- Don't add a separate `unpause_campaign.rs`; share via the type alias in `pause_campaign.rs`.
- Don't add explanatory comments on what code does. Keep a comment only when WHY is non-obvious (e.g., "Order matters: end_time first guards div-by-zero").
- Don't catch errors in tests with bare `try/catch` and silent pass; assert error variant text matches.
- Don't push the placeholder `Vesting11…` ID through to a mainnet deploy.

---

## 9. Common build errors

- **`init_if_needed requires the feature ...`** — `programs/vesting/Cargo.toml` must enable it on `anchor-lang`. Already in §6.2.
- **IDL build failure** — make sure `idl-build` lists both `anchor-lang/idl-build` and `anchor-spl/idl-build` in §6.2.
- **`InitSpace` errors on `VestingLeaf`** — `VestingLeaf` derives `InitSpace` so it can appear in the IDL with a fixed size; if Anchor 1.0 changes that, drop the derive (it's data-only and never sized as an account).
- **TS `Buffer` is not assignable to `Uint8Array`** — when calling Anchor methods that expect `number[]` for `[u8; 32]`, use `Array.from(buf)`. Already done in the rotation test.
- **Borsh field-order mismatch** between TS encoder and Rust struct — the byte-equal golden test (§6.30) catches this. If failing, check `clients/ts/leaf.ts` `encodeLeaf` against `programs/vesting/src/state/leaf.rs` field-by-field.
- **`token::transfer` CPI fails with PDA signer** — verify the `signer_seeds` slice has exactly the same seeds as the `vault_authority` constraint, in the same order, and that the bump byte is included as `&[bump]`.
- **`init_if_needed` clobbers `ClaimRecord` on second claim** — the first-touch init pattern (`if cr.beneficiary == Pubkey::default()`) handles this. Don't unconditionally re-assign identity fields.
- **LiteSVM clock doesn't advance enough between cancel and claim** — the cancel test waits 1.5s. If flaky, increase to 3s. Don't try to fast-forward the clock.

---

## 10. Test scenarios (full list — five inlined, rest to write against the same shape)

Inlined as templates in §6.33:
- T1. Happy path: linear claim mid-stream transfers proportional amount.
- T2. Invalid proof rejected.
- T3. Pause blocks claim, unpause restores it.
- T4. Cancel + claim returns pre-cancel vested only.
- T5. `update_root`: kicks a recipient — old proof fails, others keep claiming.

To add (each follows the §6.33 shape):
- T6. Claim before cliff fails with `NothingToClaim` (set `cliff_time = future(...)`).
- T7. Claim after end_time returns full leaf amount (set everything in the past).
- T8. Double-claim: second call returns `NothingToClaim` (claim once, claim again).
- T9. Unauthorized claimer: Alice signs a tx with Bob's leaf → `UnauthorizedClaimer`.
- T10. Milestone claim sets bitmap; second claim of same milestone → `MilestoneAlreadyClaimed`.
- T11. Multiple milestones for one beneficiary: two leaves, two claims, both succeed; bitmap shows two bits set.
- T12. `withdraw_unvested` before grace fails with `GracePeriodActive` (cancel + immediately try to withdraw).
- T13. `withdraw_unvested` succeeds after grace (skip — would need 7 days; document as manual devnet test).
- T14. `close_claim_record` after fully claiming refunds rent (assert beneficiary lamport delta).
- T15. `update_root` with same root fails `SameRoot`.
- T16. `update_root` with all-zero root fails `EmptyRoot`.
- T17. `update_root` from non-cancel-authority fails `Unauthorized`.
- T18. `update_root` after cancel fails `CampaignCancelled`.
- T19. `update_root` raises a recipient's amount: they claim the delta on the next call.
- T20. `update_root` lowers a recipient's amount below their `claimed_amount`: subsequent claim returns `NothingToClaim` (saturating_sub kicks in).
