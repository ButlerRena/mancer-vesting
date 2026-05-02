use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::errors::VestingError;
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
    _ctx: Context<Claim>,
    _leaf: VestingLeaf,
    _proof: Vec<[u8; 32]>,
) -> Result<()> {
    Ok(())
}
