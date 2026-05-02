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

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx:   Context<Claim>,
    leaf:  VestingLeaf,
    proof: Vec<[u8; 32]>,
) -> Result<()> {
    require!(!ctx.accounts.vesting_tree.paused, VestingError::CampaignPaused);

    require!(
        ctx.accounts.beneficiary.key() == leaf.beneficiary,
        VestingError::UnauthorizedClaimer
    );

    require!(
        leaf.start_time <= leaf.cliff_time && leaf.cliff_time <= leaf.end_time,
        VestingError::InvalidSchedule
    );
    require!(leaf.release_type <= 2, VestingError::InvalidScheduleType);

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

    let cr = &mut ctx.accounts.claim_record;
    if cr.beneficiary == Pubkey::default() {
        cr.tree              = ctx.accounts.vesting_tree.key();
        cr.beneficiary       = ctx.accounts.beneficiary.key();
        cr.claimed_amount    = 0;
        cr.milestone_bitmap  = [0u8; 32];
        cr.last_claim_at     = 0;
        cr.bump              = ctx.bumps.claim_record;
    }

    let mut milestone_idx_event: Option<u8> = None;
    if leaf.release_type == 2 {
        let byte_idx = (leaf.milestone_idx / 8) as usize;
        let bit_idx  =  leaf.milestone_idx % 8;
        let already  = (cr.milestone_bitmap[byte_idx] >> bit_idx) & 1 == 1;
        require!(!already, VestingError::MilestoneAlreadyClaimed);
        milestone_idx_event = Some(leaf.milestone_idx);
    }

    let now = Clock::get()?.unix_timestamp;
    let effective_now = match ctx.accounts.vesting_tree.cancelled_at {
        Some(c) => now.min(c),
        None    => now,
    };

    let claimable: u64 = match leaf.release_type {
        0 | 1 => {
            let total = schedule::vested(&leaf, effective_now);
            total.saturating_sub(cr.claimed_amount)
        }
        2 => {
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

    let bump = ctx.bumps.vault_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"vault_authority",
        tree_key.as_ref(),
        &[bump],
    ]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
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
